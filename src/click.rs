use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// what `click()` and the key and media taps do on a box that can't time them itself
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClickTiming {
    /// a random hold from 35 to 75 ms, the same as MAKCU's own click
    #[default]
    Makcu,
    /// a random hold from 75 to 125 ms, the same as Ferrum's own click
    Ferrum,
    Fixed(Duration),
    /// a click or tap without a hold fails on a box like that, so you have to give it one
    Off,
}

impl ClickTiming {
    fn range(self) -> Option<(Duration, Duration)> {
        match self {
            Self::Makcu => Some((Duration::from_millis(35), Duration::from_millis(75))),
            Self::Ferrum => Some((Duration::from_millis(75), Duration::from_millis(125))),
            Self::Fixed(hold) => Some((hold, hold)),
            Self::Off => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Timing {
    #[default]
    Any,
    /// fails instead of letting anybox time it from the PC
    NativeOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Click {
    /// with none, the box uses its own click timing, or the device's `ClickTiming` if the box doesn't have one
    pub hold: Option<Duration>,
    pub timing: Timing,
}

impl Click {
    pub fn hold(hold: Duration) -> Self {
        Self {
            hold: Some(hold),
            timing: Timing::Any,
        }
    }

    pub fn native_only(self) -> Self {
        Self {
            timing: Timing::NativeOnly,
            ..self
        }
    }
}

/// picks how long a click timed on the PC holds. xorshift is plenty for that and doesn't need a dependency
#[derive(Debug)]
pub(crate) struct Holds(u64);

impl Holds {
    pub(crate) fn seeded(seed: u64) -> Self {
        // xorshift gets stuck forever on a zero seed
        Self(seed | 1)
    }

    pub(crate) fn from_clock() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64);
        Self::seeded(nanos)
    }

    pub(crate) fn pick(&mut self, timing: ClickTiming) -> Option<Duration> {
        let (min, max) = timing.range()?;
        let span = (max - min).as_micros() as u64;
        if span == 0 {
            return Some(min);
        }
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        Some(min + Duration::from_micros(self.0 % (span + 1)))
    }
}

#[cfg(test)]
mod tests;
