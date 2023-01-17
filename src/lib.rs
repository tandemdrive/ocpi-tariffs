mod ocpi;
pub mod pricer;
mod restriction;
mod session;
mod tariff;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NoValidTariff,
    MissingPricingDimension,
    MissingRestrictionDimension(restriction::Restriction),
    InvalidTimeZone(chrono_tz::ParseError),
    InvalidDateTime(chrono::ParseError),
    DurationOverflow,
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Self::InvalidDateTime(err)
    }
}

impl From<chrono_tz::ParseError> for Error {
    fn from(err: chrono_tz::ParseError) -> Self {
        Self::InvalidTimeZone(err)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::DateTime;
    use rust_decimal::Decimal;

    use crate::{
        ocpi::{
            cdr::{Cdr, OcpiCdrDimension, OcpiCdrDimensionType, OcpiChargingPeriod},
            tariff::{OcpiPriceComponent, OcpiTariff, OcpiTariffElement, TariffDimensionType},
        },
        pricer::Pricer,
    };

    #[test]
    fn can_price_a_period() {
        let period = OcpiChargingPeriod {
            start_date_time: DateTime::from_str("2022-01-11 14:00:00Z").unwrap(),
            dimensions: vec![OcpiCdrDimension {
                dimension_type: OcpiCdrDimensionType::Energy,
                volume: Decimal::from_str("12.0").unwrap(),
            }],
        };

        let tariff = OcpiTariff {
            party_id: String::from("XXX"),
            county_code: String::from("NL"),
            id: String::from("XXX"),
            currency: String::from("EUR"),
            start_date_time: None,
            end_date_time: None,
            max_price: None,
            min_price: None,
            tariff_type: None,
            elements: vec![OcpiTariffElement {
                price_components: vec![OcpiPriceComponent {
                    price: Decimal::from_str("13.0").unwrap(),
                    vat: None,
                    step_size: 0,
                    component_type: TariffDimensionType::Energy,
                }],
                restrictions: None,
            }],
        };

        let cdr = Cdr {
            charging_periods: vec![period],
            tariffs: vec![tariff],
            currency: String::from("EUR"),
            start_date_time: DateTime::from_str("2022-01-11 14:00:00Z").unwrap(),
            stop_date_time: DateTime::from_str("2022-01-11 14:30:00Z").unwrap(),
            last_updated: DateTime::from_str("2022-01-01 00:00:00Z").unwrap(),
            total_cost: Decimal::from_str("0.0").unwrap(),
            total_time: Decimal::from_str("0.0").unwrap(),
            total_energy: Decimal::from_str("0.0").unwrap(),
            total_energy_cost: None,
            total_fixed_cost: None,
            total_parking_cost: None,
            total_parking_time: None,
            total_reservation_cost: None,
            total_time_cost: None,
        };

        let pricer = Pricer::new(&cdr, chrono_tz::Tz::Europe__Amsterdam).unwrap();

        let report = pricer.build_report().unwrap();

        println!("{:#?}", report);

        println!(
            "{:#?}",
            report.periods[0].dimensions[&TariffDimensionType::Energy].costs()
        );
    }
}
