use crate::backend::{Backend, Sink};
use crate::capability::{Family, Snapshot};
use crate::error::Result;
use crate::transport::Candidate;

/// opens a box of one family on a port from the list, or returns `NotFound` when that family doesn't answer there
pub(crate) type Open = fn(&Candidate, Sink) -> Result<(Box<dyn Backend>, Snapshot)>;

/// the families are tried in this order, and the order is also what keeps one family's probe away from another
/// family's box
pub(crate) const PROBES: &[(Family, Open)] = &[
    #[cfg(feature = "medius")]
    (Family::Medius, medius::open),
    #[cfg(feature = "makcu")]
    (Family::MakcuV4, makcu::open),
    #[cfg(feature = "ysc8k")]
    (Family::Ysc, ysc8k::open),
    #[cfg(feature = "ferrum")]
    (Family::Ferrum, ferrum::open),
];

#[cfg(test)]
pub(crate) mod fake;
#[cfg(feature = "ferrum")]
pub(crate) mod ferrum;
#[cfg(feature = "makcu")]
pub(crate) mod makcu;
#[cfg(feature = "medius")]
pub(crate) mod medius;
#[cfg(feature = "ysc8k")]
pub(crate) mod ysc8k;
