use std::fmt::Display;

use serde::{Deserialize, Serialize};

use super::number::Number;

/// A value of kilo watt hours.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct Kwh(Number);

impl Kwh {
    pub(crate) fn zero() -> Self {
        Self(Number::default())
    }

    /// Saturating addition
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub(crate) fn watt_hours(self) -> Number {
        self.0.saturating_mul(Number::from(1000))
    }

    pub(crate) fn from_watt_hours(num: Number) -> Self {
        Self(
            num.checked_div(Number::from(1000))
                .unwrap_or_else(|| unreachable!("divisor is non-zero")),
        )
    }

    /// Round this number to the OCPI specified amount of decimals.
    pub fn with_scale(self) -> Self {
        Self(self.0.with_scale())
    }
}

impl From<Kwh> for rust_decimal::Decimal {
    fn from(value: Kwh) -> Self {
        value.0.into()
    }
}

impl From<Kwh> for Number {
    fn from(value: Kwh) -> Self {
        value.0
    }
}

impl Display for Kwh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

/// A value of kilo watts.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Kw(Number);

impl From<Kw> for rust_decimal::Decimal {
    fn from(value: Kw) -> Self {
        value.0.into()
    }
}

/// A value of amperes.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Ampere(Number);

impl From<Number> for Ampere {
    fn from(value: Number) -> Self {
        Self(value)
    }
}

impl From<Ampere> for rust_decimal::Decimal {
    fn from(value: Ampere) -> Self {
        value.0.into()
    }
}
