pub mod cdr;
pub mod location;
pub mod tariff;

type Number = rust_decimal::Decimal;
type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    pub excl_vat: Number,
    pub incl_vat: Number,
}
