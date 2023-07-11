use serde::{Deserialize, Serialize};
use std::fmt::Display;

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

    /// Saturating addition.
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self {
            excl_vat: self.excl_vat.saturating_add(rhs.excl_vat),
            incl_vat: match (self.incl_vat, rhs.incl_vat) {
                (Some(lhs_incl_vat), Some(rhs_incl_vat)) => {
                    Some(lhs_incl_vat.saturating_add(rhs_incl_vat))
                }
                _ => None,
            },
        }
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

    /// Saturating addition
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Saturating multiplication
    pub fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    /// Apply a VAT percentage to this monetary amount.
    pub fn apply_vat(self, vat: Vat) -> Self {
        Self(self.0.saturating_mul(vat.as_fraction()))
    }

    /// Cost of a certain amount of [`Kwh`] with this price.
    pub fn kwh_cost(self, kwh: Kwh) -> Self {
        Self(self.0.saturating_mul(kwh.into()))
    }

    /// Cost of a certain amount of [`HoursDecimal`] with this price.
    pub fn time_cost(self, hours: HoursDecimal) -> Self {
        Self(self.0.saturating_mul(hours.as_num_hours_decimal()))
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

impl Vat {
    pub(crate) fn as_fraction(self) -> Number {
        self.0.checked_div(100.into()).saturating_add(1.into())
    }
}

impl Display for Vat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
