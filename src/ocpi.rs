pub mod cdr;
pub mod tariff;
pub mod location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayText {
    pub language: String,
    pub text: String,
}

pub type Number = rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    pub excl_vat: Number,
    pub incl_vat: Number,
}

pub type DateTime = chrono::DateTime<chrono::Utc>;
