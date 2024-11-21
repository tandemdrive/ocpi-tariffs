//! # OCPI Tariffs library
//!
//! Functionality to calculate the (sub)totals of a charge session. Use the
//! [`pricer::Pricer`] to perform the actual calculation.

use std::fmt;

use serde::{Deserialize, Deserializer};

/// OCPI specific structures for defining tariffs and charge sessions.
pub mod ocpi;

/// Module containing the functionality to price charge sessions with provided tariffs.
pub mod pricer;

mod restriction;
mod session;
mod tariff;

/// Module for generating human readable tariffs.
pub mod explain;

/// Module for normalizing tariffs.
pub mod normalize;

pub mod lint;

/// OCPI specific numeric types used for calculations, serializing and deserializing.
pub mod types;

type Result<T> = std::result::Result<T, Error>;

/// Possible errors when pricing a charge session.
#[derive(Debug)]
pub enum Error {
    /// No valid tariff has been found in the list of provided tariffs.
    ///
    /// A valid tariff must have a start date time before the start of the session and a end date
    /// time after the start of the session.
    ///
    /// If the session does not contain any tariffs consider providing a list of tariffs using
    /// [`pricer::Pricer::with_tariffs`].
    NoValidTariff,
    /// A numeric overflow occurred during tariff calculation.
    NumericOverflow,
    /// The CDR location did not contain a time-zone. If time zone detection was enabled and this
    /// error still occurs it means that the country specified in the CDR has multiple time-zones.
    /// Consider explicitly using a time-zone using [`pricer::Pricer::with_time_zone`].
    TimeZoneMissing,
    /// The CDR location did not contain a valid time-zone. Consider enabling time-zone detection
    /// as a fall back using [`pricer::Pricer::detect_time_zone`] or explicitly providing a time
    /// zone using [`pricer::Pricer::with_time_zone`].
    TimeZoneInvalid,
}

impl From<rust_decimal::Error> for Error {
    fn from(_: rust_decimal::Error) -> Self {
        Self::NumericOverflow
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = match self {
            Self::NoValidTariff => "No valid tariff has been found in the list of provided tariffs",
            Self::NumericOverflow => "A numeric overflow occurred during tariff calculation",
            Self::TimeZoneMissing => "No time zone could be found in the session information",
            Self::TimeZoneInvalid => "The time zone in the CDR is invalid",
        };

        f.write_str(display)
    }
}

fn null_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}
