use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul},
};

use chrono::Duration;
use rust_decimal::prelude::Zero;
use rust_decimal_macros::dec;
use serde::Deserialize;

pub type Number = rust_decimal::Decimal;
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
        let duration = Self::try_from(hours).map_err(|_| D::Error::custom("overflow"))?;
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
        let millis = value * dec!(3_600_000);
        let duration = Duration::milliseconds(millis.try_into()?);
        Ok(Self(duration))
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
                .map_err(|_| D::Error::custom("overflow"))?,
        );

        Ok(Self(duration))
    }
}

impl From<SecondsRound> for Duration {
    fn from(value: SecondsRound) -> Self {
        value.0
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct Kwh(Number);

impl Kwh {
    pub fn zero() -> Self {
        Self(Number::zero())
    }
}

impl Display for Kwh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

impl Add for Kwh {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Kwh {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0
    }
}

impl Mul<Number> for Kwh {
    type Output = Number;

    fn mul(self, rhs: Number) -> Self::Output {
        self.0 * rhs
    }
}

impl From<Number> for Kwh {
    fn from(value: Number) -> Self {
        Self(value)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Kw(Number);

impl From<Number> for Kw {
    fn from(value: Number) -> Self {
        Self(value)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Ampere(Number);

impl From<Number> for Ampere {
    fn from(value: Number) -> Self {
        Self(value)
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Copy)]
pub struct Price {
    pub excl_vat: Money,
    pub incl_vat: Money,
}

impl Price {
    pub fn zero() -> Self {
        Self {
            excl_vat: Money::zero(),
            incl_vat: Money::zero(),
        }
    }
}

impl Add for Price {
    type Output = Price;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            excl_vat: self.excl_vat + rhs.excl_vat,
            incl_vat: self.incl_vat + rhs.incl_vat,
        }
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, rhs: Self) {
        self.excl_vat = self.excl_vat + rhs.excl_vat;
        self.incl_vat = self.incl_vat + rhs.incl_vat;
    }
}

impl Default for Price {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
pub struct Money(Number);

impl Money {
    pub fn zero() -> Self {
        Self(Number::zero())
    }
}

impl Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl Add for Money {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Mul<Number> for Money {
    type Output = Money;

    fn mul(self, rhs: Number) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Money> for Number {
    type Output = Money;

    fn mul(self, rhs: Money) -> Self::Output {
        rhs * self
    }
}

impl Mul<Kwh> for Money {
    type Output = Money;

    fn mul(self, rhs: Kwh) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Mul<Money> for Kwh {
    type Output = Money;

    fn mul(self, rhs: Money) -> Self::Output {
        rhs * self
    }
}

impl Mul<Duration> for Money {
    type Output = Money;

    fn mul(self, rhs: Duration) -> Self::Output {
        Self(self.0 * (Number::from(rhs.num_milliseconds()) / dec!(3_600_000)))
    }
}

impl Mul<Money> for Duration {
    type Output = Money;

    fn mul(self, rhs: Money) -> Self::Output {
        rhs * self
    }
}

impl Mul<()> for Money {
    type Output = Money;

    fn mul(self, _: ()) -> Self::Output {
        self
    }
}

impl Mul<Money> for () {
    type Output = Money;

    fn mul(self, rhs: Money) -> Self::Output {
        rhs * self
    }
}

impl From<Money> for Number {
    fn from(value: Money) -> Self {
        value.0
    }
}
