use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul},
};

use chrono::Duration;
use rust_decimal_macros::dec;
use serde::Deserialize;

use super::{electricity::Kwh, number::Number};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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

    pub fn with_scale(self) -> Self {
        Self {
            excl_vat: self.excl_vat.with_scale(),
            incl_vat: self.incl_vat.with_scale(),
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
        Self(Number::default())
    }

    pub fn with_scale(self) -> Self {
        Self(self.0.with_scale())
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
        Money(rhs.0 * self)
    }
}

impl Mul<Kwh> for Money {
    type Output = Money;

    fn mul(self, rhs: Kwh) -> Self::Output {
        Self(self.0 * Number::from(rhs))
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
        let duration =
            self.0 * (Number::from(rhs.num_milliseconds()) / Number::from(dec!(3_600_000)));

        Self(duration)
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

impl Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize)]
#[serde(transparent)]
pub struct Vat(Number);

impl Mul<Money> for Vat {
    type Output = Money;

    fn mul(self, rhs: Money) -> Self::Output {
        let vat = (self.0 / Number::from(dec!(100))) + Number::from(dec!(1.0));
        Money(rhs.0 * vat)
    }
}

impl Mul<Vat> for Money {
    type Output = Money;
    fn mul(self, rhs: Vat) -> Self::Output {
        rhs * self
    }
}

impl Display for Vat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
