/// OCPI version 2.2.1 types
pub mod v221;

#[cfg(feature = "ocpi-v211")]
/// OCPI version 2.1.1 types
pub mod v211;

pub use v221::{cdr, tariff};
