use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul, Sub},
};

use rust_decimal_macros::dec;
use serde::Deserialize;

use super::number::Number;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Default)]
#[serde(transparent)]
pub struct Kwh(Number);

impl Kwh {
    pub fn zero() -> Self {
        Self(Number::default())
    }

    pub(crate) fn watt_hours(self) -> Number {
        self.0 * Number::from(dec!(1000.0))
    }

    pub(crate) fn from_watt_hours(num: Number) -> Self {
        Self(num / dec!(1000.0).into())
    }

    pub fn with_scale(self) -> Self {
        Self(self.0.with_scale())
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

impl Sub for Kwh {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<Number> for Kwh {
    type Output = Self;

    fn mul(self, rhs: Number) -> Self::Output {
        Self(self.0 * rhs)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Kw(Number);

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Ampere(Number);

impl From<Number> for Ampere {
    fn from(value: Number) -> Self {
        Self(value.into())
    }
}
