use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crate::backend::{Backend, Signal, Sink};
use crate::capability::{Action, Capabilities, Family, FerrumApi, Info, Snapshot, StableId};
use crate::click::{ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::{Error, Result};
use crate::family::{self, Open};
use crate::transport::os::OsSystem;
use crate::transport::{Candidate, System, is_ch343, is_ysc};

/// how long `scan` waits for each box it opened to let go of its port
const CLOSE_DEADLINE: Duration = Duration::from_secs(1);

/// connects to the first box found, in port order
pub fn connect() -> Result<Device> {
    Connect::auto().open()
}

/// every port a search looks at and what's on it, in port order, so a picker can show the boxes you can use and
/// why you can't use the others. it opens, describes and closes every box, so it's slow (a YSC search takes about
/// 2 s a port), `candidates()` is the cheap one to call often
pub fn scan() -> Vec<Scanned> {
    Connect::auto().scan()
}

/// one port a scan looked at. `result` is the box on it, or the error a search of just that port gives. mostly
/// that's busy (the box you already have open shows up as busy too, so leave its port out), not allowed, too old,
/// `NoBoxCan` for a box that can't do what `can` asked, or `NotFound` when nothing answered
#[derive(Debug)]
#[non_exhaustive]
pub struct Scanned {
    pub port: String,
    pub result: Result<Found>,
}

/// the ports a search would look at, in port order, without opening any of them or sending them a byte. it can't
/// tell whether a box is on one, only where one could be, so it's cheap enough to call every second and run `scan`
/// when the list changes. a Ferrum has no USB id anybox knows, so its port is never listed here. a YSC V1 sits behind the same
/// chip as a Medius or a MAKCU, so its port shows up here, but only `.family(Family::Ysc)` ever looks for a V1 on it
pub fn candidates() -> Vec<String> {
    Connect::auto().candidates()
}

/// one box a scan saw
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Found {
    pub port: String,
    pub info: Info,
    /// what it can do, the same as `device.snapshot().capabilities` would say right after opening it
    pub capabilities: Capabilities,
}

#[derive(Clone, Debug, Default)]
pub struct Connect {
    port: Option<PathBuf>,
    family: Option<Family>,
    id: Option<StableId>,
    click_timing: ClickTiming,
    can: Vec<Action>,
    ferrum_api: Option<FerrumApi>,
}

impl Connect {
    /// asks CH343 (1A86:55D3) ports and YSC V2 left ports (1A86:FE0C with serial TOWMCULEFT). a Ferrum is only
    /// asked with `.family(Family::Ferrum)` (the other ports then) or `Connect::port`, and a YSC V1 only with
    /// `.family(Family::Ysc)`, which then asks every CH343 port at up to 9 rates
    pub fn auto() -> Self {
        Self::default()
    }

    pub fn port(path: impl Into<PathBuf>) -> Self {
        Self {
            port: Some(path.into()),
            ..Self::default()
        }
    }

    /// skips a box that can't do this, checked once when it opens, so with a keyboard box and a mouse box
    /// plugged in `.can(Action::Move)` opens the mouse one. call it again for each thing it has to do.
    /// `Action::Connect` matches no box
    pub fn can(mut self, action: Action) -> Self {
        self.can.push(action);
        self
    }

    /// only tries this family, so the port never gets anything meant for another box
    pub fn family(self, family: Family) -> Self {
        Self {
            family: Some(family),
            ..self
        }
    }

    /// only a Ferrum on this API, so asking for Software never ends up on the box's own legacy port. on Windows a
    /// Ferrum search without a port already only takes Software, so ask for Legacy here if that's the one you
    /// want. other boxes don't care about it
    pub fn ferrum_api(self, api: FerrumApi) -> Self {
        Self {
            ferrum_api: Some(api),
            ..self
        }
    }

    /// only the box with this id, whatever port it's on. a box with no id (like a MAKCU whose USB serial can't be
    /// trusted) can't be picked this way, only by its port
    pub fn id(self, id: StableId) -> Self {
        Self {
            id: Some(id),
            ..self
        }
    }

    /// what `click()` does on a box with no click of its own
    pub fn click_timing(self, click_timing: ClickTiming) -> Self {
        Self {
            click_timing,
            ..self
        }
    }

    /// the ports `scan` and `open` would look at with these filters, see `candidates()`. with
    /// `.family(Family::Ferrum)` that's every port that isn't a CH343 or a YSC port, plus on Windows the ones with no
    /// USB id
    pub fn candidates(&self) -> Vec<String> {
        self.candidates_in(&|| OsSystem.ports())
    }

    pub(crate) fn candidates_in(&self, ports: &dyn Fn() -> Vec<Candidate>) -> Vec<String> {
        self.listed(ports())
            .into_iter()
            .map(|candidate| candidate.path.display().to_string())
            .collect()
    }

    /// the candidates in port order, before anything gets opened
    fn listed(&self, listed: Vec<Candidate>) -> Vec<Candidate> {
        let mut candidates: Vec<Candidate> = match &self.port {
            // a port the caller names gets asked whatever its USB id is, because the caller knows what's on it
            Some(path) => match listed.into_iter().find(|candidate| &candidate.path == path) {
                Some(candidate) => vec![candidate],
                None => vec![Candidate {
                    path: path.clone(),
                    usb: None,
                }],
            },
            None => listed
                .into_iter()
                .filter(|candidate| self.lists(candidate) && self.might_be(candidate))
                .collect(),
        };
        candidates.sort_by(|a, b| port_order(&a.path, &b.path));
        if self.family == Some(Family::Ysc) {
            // a V2 port answers at once, so it goes before the CH343 ports that each take a whole rate sweep
            candidates.sort_by_key(|candidate| !candidate.usb.as_ref().is_some_and(is_ysc));
        }
        candidates
    }

    pub fn open(self) -> Result<Device> {
        self.open_in(&|| OsSystem.ports(), family::PROBES)
    }

    pub(crate) fn open_in(
        self,
        ports: &dyn Fn() -> Vec<Candidate>,
        probes: &[(Family, Open)],
    ) -> Result<Device> {
        let mut search = self.search(ports, probes);
        match search.opened.take() {
            Some(found) => Device::start(Start {
                backend: found.backend,
                snapshot: found.snapshot,
                signals: found.signals,
                incoming: found.incoming,
                click_timing: self.click_timing,
                holds: Holds::from_clock(),
                port: found.port,
            }),
            None => Err(search.error()),
        }
    }

    /// what happened on every port this search looks at, in port order, see `anybox::scan`
    pub fn scan(self) -> Vec<Scanned> {
        self.scan_in(&|| OsSystem.ports(), family::PROBES)
    }

    pub(crate) fn scan_in(
        &self,
        ports: &dyn Fn() -> Vec<Candidate>,
        probes: &[(Family, Open)],
    ) -> Vec<Scanned> {
        self.listed(ports())
            .iter()
            .map(|candidate| {
                let port = candidate.path.display().to_string();
                let mut search = Search {
                    tried: vec![port.clone()],
                    ..Search::default()
                };
                let result = match self.probe(candidate, ports, probes, &mut search) {
                    Some(opened) => {
                        opened.backend.shutdown(CLOSE_DEADLINE);
                        Ok(Found {
                            capabilities: self.capabilities(&opened.snapshot),
                            port: opened.port,
                            info: opened.snapshot.info,
                        })
                    }
                    None => Err(search.error()),
                };
                Scanned { port, result }
            })
            .collect()
    }

    /// the boxes a scan found, for the tests that only care about those
    #[cfg(test)]
    pub(crate) fn found_in(
        &self,
        ports: &dyn Fn() -> Vec<Candidate>,
        probes: &[(Family, Open)],
    ) -> Vec<Found> {
        self.scan_in(ports, probes)
            .into_iter()
            .filter_map(|scanned| scanned.result.ok())
            .collect()
    }

    /// the first box on the candidate ports in port order that passes the filters, and what went wrong on the
    /// ports before it
    pub(crate) fn search(
        &self,
        ports: &dyn Fn() -> Vec<Candidate>,
        probes: &[(Family, Open)],
    ) -> Search {
        let mut search = Search::default();
        for candidate in &self.listed(ports()) {
            search.tried.push(candidate.path.display().to_string());
            if let Some(opened) = self.probe(candidate, ports, probes, &mut search) {
                search.opened = Some(opened);
                break;
            }
        }
        search
    }

    /// what the device will say it can do, with the clicks and taps the core builds on the PC where the box has
    /// none, so a check here agrees with `device.snapshot()`
    fn capabilities(&self, snapshot: &Snapshot) -> Capabilities {
        snapshot
            .capabilities
            .clone()
            .with_host_taps(self.click_timing)
    }

    /// rules out what the port listing alone already can, so a box nobody asked for doesn't get opened
    fn might_be(&self, candidate: &Candidate) -> bool {
        match &self.id {
            Some(StableId::UsbSerial(serial)) => {
                candidate.usb.as_ref().and_then(|usb| usb.serial.as_ref()) == Some(serial)
            }
            _ => true,
        }
    }

    /// Medius and MAKCU are always behind a CH343. a Ferrum is behind a CP210x or the Ferrum app's virtual port,
    /// and a CP210x is on lots of unrelated devices, so those ports only get text sent to them when a Ferrum was
    /// asked for
    fn lists(&self, candidate: &Candidate) -> bool {
        let ferrum = self.family == Some(Family::Ferrum);
        match &candidate.usb {
            // a YSC V1 sits behind a CH343 too
            Some(usb) if is_ch343(usb) => !ferrum,
            Some(usb) if is_ysc(usb) => {
                self.family.is_none_or(|family| family == Family::Ysc)
                    && usb.serial.as_deref() == Some("TOWMCULEFT")
            }
            Some(_) => ferrum,
            // the virtual port presumably shows up with no USB id, and the app only runs on Windows
            None => ferrum && cfg!(windows),
        }
    }

    /// on Windows the app's port is the one with everything, so a search there never picks the box's own legacy
    /// port by accident. Linux has no app, so it takes whatever answers
    fn wanted_ferrum_api(&self) -> Option<FerrumApi> {
        self.ferrum_api
            .or_else(|| (cfg!(windows) && self.port.is_none()).then_some(FerrumApi::Software))
    }

    fn wants(&self, family: Family, candidate: &Candidate) -> bool {
        // a Ferrum gets no USB id we know of yet, so it's only asked when someone asked for one
        if family == Family::Ferrum && self.port.is_none() && self.family != Some(family) {
            return false;
        }
        // a YSC V1 has no USB id of its own (the one we've seen sits behind a CH343 like a Medius or a MAKCU), so a port
        // that isn't a V2 one only gets the YSC rate sweep when someone asked for a YSC
        if family == Family::Ysc
            && self.family != Some(family)
            && !candidate.usb.as_ref().is_some_and(is_ysc)
        {
            return false;
        }
        // every family asks a named port, so another box there comes back as WrongBox and not NotFound
        let by_id = self.port.is_some()
            || match &self.id {
                // only a Medius gives a MAC (in its VERSION reply), and only a MAKCU has a USB serial we trust
                Some(StableId::Mac(_)) => family == Family::Medius,
                Some(StableId::UsbSerial(_)) => family == Family::MakcuV4,
                None => true,
            };
        by_id && self.family.is_none_or(|wanted| wanted == family)
    }

    fn probe(
        &self,
        candidate: &Candidate,
        ports: &dyn Fn() -> Vec<Candidate>,
        probes: &[(Family, Open)],
        search: &mut Search,
    ) -> Option<Opened> {
        let port = candidate.path.display().to_string();
        for (family, open) in probes {
            if !self.wants(*family, candidate) {
                continue;
            }
            let (signals, incoming) = mpsc::channel();
            match open(candidate, Sink(signals.clone())) {
                Ok((backend, mut snapshot)) => {
                    snapshot.info.port.clone_from(&port);
                    if let Some(expected) = &self.id
                        && snapshot.info.id.as_ref() != Some(expected)
                    {
                        backend.shutdown(CLOSE_DEADLINE);
                        if self.port.is_some() {
                            search.skipped.push(Error::WrongBox {
                                port,
                                expected: expected.clone(),
                                found: snapshot.info.id,
                            });
                        }
                        return None;
                    }
                    if let (Some(expected), Some(found)) =
                        (self.wanted_ferrum_api(), snapshot.info.ferrum_api)
                        && expected != found
                    {
                        backend.shutdown(CLOSE_DEADLINE);
                        search.skipped.push(Error::WrongFerrumApi {
                            port,
                            expected,
                            found,
                        });
                        return None;
                    }
                    let capabilities = self.capabilities(&snapshot);
                    let missing: Vec<Action> = self
                        .can
                        .iter()
                        .copied()
                        .filter(|action| !capabilities.get(*action).is_supported())
                        .collect();
                    if !missing.is_empty() {
                        backend.shutdown(CLOSE_DEADLINE);
                        search.incapable.push((
                            Found {
                                port,
                                info: snapshot.info,
                                capabilities,
                            },
                            missing,
                        ));
                        return None;
                    }
                    return Some(Opened {
                        port,
                        backend,
                        snapshot,
                        signals,
                        incoming,
                    });
                }
                Err(Error::NotFound { .. }) => {}
                // a busy or forbidden port is the same for every family, and once a box said what it is there's
                // no point asking the other families either
                Err(error) => {
                    search.skipped.push(error);
                    return None;
                }
            }
        }
        // a port that isn't there at all isn't a box of the wrong family. the ports get listed again because one
        // can go away while it's being asked
        if let (Some(expected), Some(_)) = (self.family, &self.port)
            && ports().iter().any(|listed| listed.path == candidate.path)
        {
            search.skipped.push(Error::WrongFamily {
                port,
                expected,
                found: None,
            });
        }
        None
    }
}

pub(crate) struct Opened {
    port: String,
    backend: Box<dyn Backend>,
    snapshot: Snapshot,
    signals: Sender<Signal>,
    incoming: Receiver<Signal>,
}

#[derive(Default)]
pub(crate) struct Search {
    opened: Option<Opened>,
    skipped: Vec<Error>,
    /// boxes that opened but can't do what `can` asked
    incapable: Vec<(Found, Vec<Action>)>,
    tried: Vec<String>,
}

impl Search {
    /// when no box was found, the error that helps the most. an update to install comes first, then a port
    /// someone else has open, then one this user isn't allowed to open, then anything else, and NotFound last
    fn error(self) -> Error {
        let rank = |error: &Error| match error {
            Error::FirmwareTooOld { .. } | Error::LibraryTooOld { .. } => 0,
            Error::PortBusy { .. } => 1,
            Error::PermissionDenied { .. } => 2,
            _ => 3,
        };
        let best = self.skipped.into_iter().min_by_key(rank);
        match best {
            Some(error) if rank(&error) < 3 => error,
            // boxes answered and only fell short of `can`, which says more than the other misses
            _ if !self.incapable.is_empty() => Error::NoBoxCan {
                boxes: self.incapable,
            },
            Some(error) => error,
            None => Error::NotFound { tried: self.tried },
        }
    }
}

/// `COM3` before `COM11`, so it compares the name without its trailing number first and then that number
fn port_order(a: &Path, b: &Path) -> Ordering {
    fn split(path: &Path) -> (String, Option<u64>) {
        let name = path.display().to_string();
        let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
        (stem.to_string(), name[stem.len()..].parse().ok())
    }
    split(a).cmp(&split(b)).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests;
