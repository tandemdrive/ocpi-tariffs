use std::fmt::Display;

use chrono::Duration;
use rust_decimal_macros::dec;
use serde::Deserialize;

use super::number::Number;

pub type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct HoursDecimal(Duration);

impl<'de> Deserialize<'de> for HoursDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let hours = <Number as serde::Deserialize>::deserialize(deserializer)?;
        let duration = Self::try_from(hours).map_err(|_e| D::Error::custom("overflow"))?;
        Ok(duration)
    }
}

impl From<HoursDecimal> for Duration {
    fn from(value: HoursDecimal) -> Self {
        value.0
    }
}

impl From<Duration> for HoursDecimal {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl TryFrom<Number> for HoursDecimal {
    type Error = rust_decimal::Error;

    fn try_from(value: Number) -> Result<Self, Self::Error> {
        let millis = value * Number::from(dec!(3_600_000));
        let duration = Duration::milliseconds(millis.try_into()?);
        Ok(Self(duration))
    }
}

impl HoursDecimal {
    pub fn zero() -> Self {
        Self(Duration::zero())
    }
}

impl Default for HoursDecimal {
    fn default() -> Self {
        Self::zero()
    }
}

impl Display for HoursDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.0;
        let seconds = duration.num_seconds() % 60;
        let minutes = (duration.num_seconds() / 60) % 60;
        let hours = (duration.num_seconds() / 60) / 60;
        write!(f, "{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SecondsRound(Duration);

impl<'de> Deserialize<'de> for SecondsRound {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let seconds = <u64 as serde::Deserialize>::deserialize(deserializer)?;
        let duration = Duration::seconds(
            seconds
                .try_into()
                .map_err(|_e| D::Error::custom("overflow"))?,
        );

        Ok(Self(duration))
    }
}

impl From<SecondsRound> for Duration {
    fn from(value: SecondsRound) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OcpiDate(chrono::NaiveDate);

impl<'de> Deserialize<'de> for OcpiDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let s = <String as Deserialize>::deserialize(deserializer)?;
        let date = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(D::Error::custom)?;

        Ok(Self(date))
    }
}

impl From<OcpiDate> for chrono::NaiveDate {
    fn from(value: OcpiDate) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct OcpiTime(chrono::NaiveTime);

impl<'de> Deserialize<'de> for OcpiTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let s = <String as Deserialize>::deserialize(deserializer)?;
        let date = chrono::NaiveTime::parse_from_str(&s, "%H:%M").map_err(D::Error::custom)?;

        Ok(Self(date))
    }
}

impl From<OcpiTime> for chrono::NaiveTime {
    fn from(value: OcpiTime) -> Self {
        value.0
    }
}
