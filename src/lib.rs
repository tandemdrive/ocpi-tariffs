pub mod ocpi;
pub mod pricer;
mod restriction;
mod session;
mod tariff;
mod types;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NoValidTariff,
    MissingPricingDimension,
    MissingRestrictionDimension(restriction::Restriction),
    InvalidTimeZone(chrono_tz::ParseError),
    InvalidDateTime(chrono::ParseError),
    DurationOverflow,
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Self::InvalidDateTime(err)
    }
}

impl From<chrono_tz::ParseError> for Error {
    fn from(err: chrono_tz::ParseError) -> Self {
        Self::InvalidTimeZone(err)
    }
}
