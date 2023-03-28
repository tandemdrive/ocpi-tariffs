use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) struct Number(rust_decimal::Decimal);

impl Number {
    pub(crate) fn ceil(self) -> Self {
        Self(self.0.ceil())
    }

    pub(crate) fn with_scale(mut self) -> Self {
        self.0.rescale(4);
        self
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut decimal = <rust_decimal::Decimal as Deserialize>::deserialize(deserializer)?;
        decimal.rescale(4);
        Ok(Self(decimal))
    }
}

impl From<rust_decimal::Decimal> for Number {
    fn from(value: rust_decimal::Decimal) -> Self {
        Self(value)
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self(value.into())
    }
}

impl From<u64> for Number {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl TryFrom<Number> for i64 {
    type Error = rust_decimal::Error;

    fn try_from(value: Number) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Mul for Number {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_mul(rhs.0))
    }
}

impl Div for Number {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
