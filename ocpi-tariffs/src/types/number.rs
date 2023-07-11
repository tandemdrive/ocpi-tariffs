use std::fmt::Display;

use serde::{Deserialize, Deserializer, Serialize};

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

    pub(crate) fn checked_div(self, other: Self) -> Self {
        Self(self.0.checked_div(other.0).expect("divide by zero"))
    }

    pub(crate) fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    pub(crate) fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub(crate) fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
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

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut decimal = self.0;

        decimal.rescale(4);
        decimal.normalize_assign();

        Serialize::serialize(&decimal, serializer)
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

impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Self(value.into())
    }
}

impl TryFrom<Number> for i64 {
    type Error = rust_decimal::Error;

    fn try_from(value: Number) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
