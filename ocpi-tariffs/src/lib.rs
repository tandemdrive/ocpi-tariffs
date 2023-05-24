#![deny(missing_docs)]
//! # OCPI Tariffs library
//!
//! Functionality to calculate the (sub)totals of a charge session. Use the
//! [`pricer::Pricer`] to perform the actual calculation.

use std::fmt;

/// OCPI specific structures for defining tariffs and charge sessions.
pub mod ocpi;
/// Module containing the functionality to price charge sessions with provided tariffs.
pub mod pricer;

mod restriction;
mod session;
mod tariff;

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
    NoValidTariff,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("No valid tariff has been found in the list of provided tariffs")
    }
}
