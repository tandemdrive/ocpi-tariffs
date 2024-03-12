use std::fmt::Display;

use chrono::Duration;
use serde::{Deserialize, Serialize, Serializer};

use crate::Error;

use super::number::Number;

const SECS_IN_MIN: i64 = 60;
const MINS_IN_HOUR: i64 = 60;
const MILLIS_IN_SEC: i64 = 1000;

/// A `chrono` UTC date time.
pub type DateTime = chrono::DateTime<chrono::Utc>;

/// A generic duration type that converts from and to a decimal amount of hours.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct HoursDecimal(Duration);

impl<'de> Deserialize<'de> for HoursDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let hours = Number::deserialize(deserializer)?;
        let duration = Self::from_hours_number(hours).map_err(|_e| D::Error::custom("overflow"))?;
        Ok(duration)
    }
}

impl Serialize for HoursDecimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let hours = self.as_num_hours_number();
        hours.serialize(serializer)
    }
}

impl Display for HoursDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl HoursDecimal {
    pub(crate) fn zero() -> Self {
        Self(Duration::zero())
    }

    pub(crate) fn as_num_seconds_number(&self) -> Number {
        Number::from(self.0.num_milliseconds())
            .checked_div(Number::from(MILLIS_IN_SEC))
            .unwrap_or_else(|| unreachable!("divisor is non-zero"))
    }

    /// Convert into decimal representation.
    pub fn as_num_hours_decimal(&self) -> rust_decimal::Decimal {
        self.as_num_hours_number().into()
    }

    pub(crate) fn as_num_hours_number(&self) -> Number {
        Number::from(self.0.num_milliseconds())
            .checked_div(Number::from(MILLIS_IN_SEC * SECS_IN_MIN * MINS_IN_HOUR))
            .unwrap_or_else(|| unreachable!("divisor is non-zero"))
    }

    pub(crate) fn from_seconds_number(seconds: Number) -> Result<Self, Error> {
        let millis = seconds.saturating_mul(Number::from(MILLIS_IN_SEC));

        Ok(Self(
            Duration::try_milliseconds(millis.try_into()?).ok_or(Error::NumericOverflow)?,
        ))
    }

    pub(crate) fn from_hours_number(hours: Number) -> Result<Self, Error> {
        let millis = hours.saturating_mul(Number::from(MILLIS_IN_SEC * SECS_IN_MIN * MINS_IN_HOUR));

        Ok(Self(
            Duration::try_milliseconds(millis.try_into()?).ok_or(Error::NumericOverflow)?,
        ))
    }

    /// Saturating subtraction.
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.checked_sub(&other.0).unwrap_or_else(Duration::zero))
    }

    /// Saturating addition.
    pub fn saturating_add(self, other: Self) -> Self {
        Self(
            self.0
                .checked_add(&other.0)
                .unwrap_or_else(Duration::max_value),
        )
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
        use serde::de::Error as DeError;

        let seconds: i64 = u64::deserialize(deserializer)?
            .try_into()
            .map_err(|_| DeError::custom(Error::NumericOverflow))?;

        let duration = Duration::try_seconds(seconds)
            .ok_or_else(|| DeError::custom(Error::NumericOverflow))?;

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

/// Days of the week.
#[derive(Copy, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DayOfWeek {
    /// Monday
    Monday,
    /// Tuesday
    Tuesday,
    /// Wednesday
    Wednesday,
    /// Thursday
    Thursday,
    /// Friday
    Friday,
    /// Saturday
    Saturday,
    /// Sunday
    Sunday,
}

impl From<DayOfWeek> for chrono::Weekday {
    fn from(day: DayOfWeek) -> Self {
        match day {
            DayOfWeek::Monday => Self::Mon,
            DayOfWeek::Tuesday => Self::Tue,
            DayOfWeek::Wednesday => Self::Wed,
            DayOfWeek::Thursday => Self::Thu,
            DayOfWeek::Friday => Self::Fri,
            DayOfWeek::Saturday => Self::Sat,
            DayOfWeek::Sunday => Self::Sun,
        }
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
        let hours: HoursDecimal = Duration::try_minutes(0).unwrap().into();
        let number: Number = hours.as_num_hours_number();
        assert_eq!(number, Number::from(dec!(0.0)));
    }

    #[test]
    fn thirty_minutes_should_be_fraction_of_hour() {
        let hours: HoursDecimal = Duration::try_minutes(30).unwrap().into();
        let number: Number = hours.as_num_hours_number();
        assert_eq!(number, Number::from(dec!(0.5)));
    }

    #[test]
    fn sixty_minutes_should_be_fraction_of_hour() {
        let hours: HoursDecimal = Duration::try_minutes(60).unwrap().into();
        let number: Number = hours.as_num_hours_number();
        assert_eq!(number, Number::from(dec!(1.0)));
    }

    #[test]
    fn ninety_minutes_should_be_fraction_of_hour() {
        let hours: HoursDecimal = Duration::try_minutes(90).unwrap().into();
        let number: Number = hours.as_num_hours_number();
        assert_eq!(number, Number::from(dec!(1.5)));
    }
}
