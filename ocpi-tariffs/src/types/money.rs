use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul},
};

use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::{electricity::Kwh, number::Number, time::HoursDecimal};

/// A price consisting of a value including VAT, and a value excluding VAT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Price {
    /// The price excluding VAT.
    pub excl_vat: Money,
    #[serde(default)]
    /// The price including VAT.
    ///
    /// If no vat is applicable this value will be equal to the excl_vat.
    ///
    /// If no vat could be determined (tariff is 2.1.1) this value will be `None`.
    pub incl_vat: Option<Money>,
}

impl Price {
    pub(crate) fn zero() -> Self {
        Self {
            excl_vat: Money::zero(),
            incl_vat: Some(Money::zero()),
        }
    }

    /// Round this number to the OCPI specified amount of decimals.
    pub fn with_scale(self) -> Self {
        Self {
            excl_vat: self.excl_vat.with_scale(),
            incl_vat: self.incl_vat.map(Money::with_scale),
        }
    }
}

impl Add for Price {
    type Output = Price;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            excl_vat: self.excl_vat + rhs.excl_vat,
            incl_vat: match (self.incl_vat, rhs.incl_vat) {
                (Some(lhs_incl_vat), Some(rhs_incl_vat)) => Some(lhs_incl_vat + rhs_incl_vat),
                _ => None,
            },
        }
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, rhs: Self) {
        self.excl_vat = self.excl_vat + rhs.excl_vat;

        self.incl_vat = match (self.incl_vat, rhs.incl_vat) {
            (Some(lhs_incl_vat), Some(rhs_incl_vat)) => Some(lhs_incl_vat + rhs_incl_vat),
            _ => None,
        };
    }
}

impl Default for Price {
    fn default() -> Self {
        Self::zero()
    }
}

/// A monetary amount, the currency is dependant on the specified tariff.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
pub struct Money(Number);

impl Money {
    pub(crate) fn zero() -> Self {
        Self(Number::default())
    }

    /// Round this number to the OCPI specified amount of decimals.
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

impl Mul<HoursDecimal> for Money {
    type Output = Money;

    fn mul(self, rhs: HoursDecimal) -> Self::Output {
        let duration =
            self.0 * (Number::from(rhs.0.num_milliseconds()) / Number::from(dec!(3_600_000)));

        Self(duration)
    }
}

impl Mul<Money> for HoursDecimal {
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

/// A VAT percentage.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
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
