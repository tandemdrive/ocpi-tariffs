pub mod ocpi;
pub mod pricer;
mod restriction;
mod session;
mod tariff;
pub mod types;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NoValidTariff,
}
