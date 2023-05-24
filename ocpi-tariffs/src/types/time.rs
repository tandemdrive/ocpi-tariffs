use std::fmt::Display;

use chrono::Duration;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize, Serializer};

use super::number::Number;

/// A `chrono` UTC date time.
pub type DateTime = chrono::DateTime<chrono::Utc>;

/// A generic duration type that converts from and to a decimal amount of hours.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct HoursDecimal(pub(crate) Duration);

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

impl Serialize for HoursDecimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for HoursDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SECS_IN_MIN: i64 = 60;
        const MINS_IN_HOUR: i64 = 60;

        let duration = self.0;
        let seconds = duration.num_seconds() % SECS_IN_MIN;
        let minutes = (duration.num_seconds() / SECS_IN_MIN) % MINS_IN_HOUR;
        let hours = duration.num_seconds() / (SECS_IN_MIN * MINS_IN_HOUR);

        write!(f, "{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds)
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

impl From<&HoursDecimal> for Number {
    fn from(value: &HoursDecimal) -> Self {
        use rust_decimal::Decimal;

        let seconds: Decimal = value.0.num_seconds().into();
        let hours = seconds / dec!(3600);

        hours.into()
    }
}

impl From<HoursDecimal> for Number {
    fn from(value: HoursDecimal) -> Self {
        (&value).into()
    }
}

impl HoursDecimal {
    pub(crate) fn zero() -> Self {
        Self(Duration::zero())
    }
}

impl Default for HoursDecimal {
    fn default() -> Self {
        Self::zero()
    }
}

/// A generic duration type that converts from and to a integer amount of seconds.
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

impl Serialize for SecondsRound {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let seconds = self.0.num_seconds();
        serializer.serialize_i64(seconds)
    }
}

impl From<SecondsRound> for Duration {
    fn from(value: SecondsRound) -> Self {
        value.0
    }
}

/// A OCPI specific local date, without a time.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize)]
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

/// A OCPI specific local time, without a date.
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Serialize)]
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

#[cfg(test)]
mod hour_decimal_tests {
    use chrono::Duration;
    use rust_decimal_macros::dec;

    use crate::types::number::Number;

    use super::HoursDecimal;

    #[test]
    fn zero_minutes_should_be_zero_hours() {
        let hours: HoursDecimal = Duration::minutes(0).into();
        let number: Number = hours.into();
        assert_eq!(number, Number::from(dec!(0.0)));
    }

    #[test]
    fn thirty_minutes_should_be_fraction_of_hour() {
        let hours: HoursDecimal = Duration::minutes(30).into();
        let number: Number = hours.into();
        assert_eq!(number, Number::from(dec!(0.5)));
    }

    #[test]
    fn sixty_minutes_should_be_fraction_of_hour() {
        let hours: HoursDecimal = Duration::minutes(60).into();
        let number: Number = hours.into();
        assert_eq!(number, Number::from(dec!(1.0)));
    }

    #[test]
    fn ninety_minutes_should_be_fraction_of_hour() {
        let hours: HoursDecimal = Duration::minutes(90).into();
        let number: Number = hours.into();
        assert_eq!(number, Number::from(dec!(1.5)));
    }
}
