use std::{collections::HashMap, ops::Mul};

use crate::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff, tariff::TariffDimensionType},
    session::ChargePeriod,
    tariff::{PriceComponent, Tariffs},
    types::{Kwh, Money, Price},
};

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use rust_decimal_macros::dec;

use crate::{session::ChargeSession, Result};
use rust_decimal::Decimal;

pub struct Pricer {
    session: ChargeSession,
    tariffs: Tariffs,
}

impl Pricer {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Self {
        Self {
            session: ChargeSession::new(cdr, local_timezone),
            tariffs: Tariffs::new(&cdr.tariffs),
        }
    }

    pub fn with_tariffs(cdr: &Cdr, tariffs: &[OcpiTariff], local_timezone: Tz) -> Self {
        Self {
            session: ChargeSession::new(cdr, local_timezone),
            tariffs: Tariffs::new(tariffs),
        }
    }

    pub fn build_report(&self) -> Result<Report> {
        let tariff = self.tariffs.active_tariff(self.session.start_date_time)?;

        let mut report = Report::new();
        let mut periods = Vec::new();

        for period in self.session.periods.iter() {
            let components = tariff.active_components(period);

            let dimensions = Dimensions {
                flat: DimensionReport::new(components.flat, ()),
                time: DimensionReport::new(
                    components.time,
                    period.period_data.duration.unwrap_or_else(Duration::zero),
                ),
                parking_time: DimensionReport::new(
                    components.parking,
                    period
                        .period_data
                        .parking_duration
                        .unwrap_or_else(Duration::zero),
                ),
                energy: DimensionReport::new(
                    components.energy,
                    period.period_data.energy.unwrap_or_else(Kwh::zero),
                ),
            };

            periods.push(PeriodReport::new(tariff.tariff_index, period, dimensions));
        }

        for period in &periods {
            report.total_cost += period.cost();

            report.total_energy += period.dimensions.energy.volume;
            report.total_energy_cost += period.dimensions.energy.cost();

            report.total_time_cost += period.dimensions.time.cost();

            report.total_fixed_cost += period.dimensions.flat.cost();

            report.total_parking_time =
                report.total_parking_time + period.dimensions.parking_time.volume;
            report.total_parking_cost += period.dimensions.parking_time.cost();
        }

        let start_time = periods.first().unwrap().start_date_time;
        let stop_time = periods.last().unwrap().end_date_time;

        // total time in hours
        report.total_time = (stop_time - start_time).into();

        report.periods = periods;

        Ok(report)
    }
}

#[derive(Debug)]
pub struct Report {
    pub periods: Vec<PeriodReport>,
    pub total_cost: Price,
    pub total_time_cost: Price,
    pub total_time: Duration,
    pub total_parking_cost: Price,
    pub total_parking_time: Duration,
    pub total_energy_cost: Price,
    pub total_energy: Kwh,
    pub total_fixed_cost: Price,
    pub total_reservation_cost: Price,
}

impl Report {
    fn new() -> Self {
        Self {
            periods: Vec::new(),
            total_cost: Price::zero(),
            total_time_cost: Price::zero(),
            total_time: Duration::zero(),
            total_parking_cost: Price::zero(),
            total_parking_time: Duration::zero(),
            total_energy_cost: Price::zero(),
            total_energy: Kwh::zero(),
            total_fixed_cost: Price::zero(),
            total_reservation_cost: Price::zero(),
        }
    }
}

#[derive(Debug)]
pub struct PeriodReport {
    pub tariff_index: usize,
    pub start_date_time: DateTime<Utc>,
    pub end_date_time: DateTime<Utc>,
    pub dimensions: Dimensions,
}

impl PeriodReport {
    fn new(tariff_index: usize, period: &ChargePeriod, dimensions: Dimensions) -> Self {
        Self {
            tariff_index,
            start_date_time: period.start_instant.date_time,
            end_date_time: period.end_instant.date_time,
            dimensions,
        }
    }

    fn cost(&self) -> Price {
        self.dimensions.time.cost()
            + self.dimensions.parking_time.cost()
            + self.dimensions.flat.cost()
            + self.dimensions.energy.cost()
    }
}

#[derive(Debug)]
pub struct Dimensions {
    pub flat: DimensionReport<()>,
    pub energy: DimensionReport<Kwh>,
    pub time: DimensionReport<Duration>,
    pub parking_time: DimensionReport<Duration>,
}

#[derive(Debug)]
pub struct DimensionReport<V> {
    pub price: Option<PriceComponent>,
    pub volume: V,
}

impl<V> DimensionReport<V> {
    pub fn new(price_component: Option<PriceComponent>, volume: V) -> Self {
        Self {
            price: price_component,
            volume,
        }
    }
}

impl<V> DimensionReport<V>
where
    V: Mul<Money, Output = Money> + Copy,
{
    pub fn cost(&self) -> Price {
        Price {
            incl_vat: self.cost_incl_vat(),
            excl_vat: self.cost_excl_vat(),
        }
    }

    fn cost_excl_vat(&self) -> Money {
        let price = self.price.map(|c| c.price).unwrap_or_default();
        self.volume * price
    }

    fn cost_incl_vat(&self) -> Money {
        if let Some(vat) = self.price.and_then(|c| c.vat) {
            self.cost_excl_vat() * ((vat / dec!(100)) + dec!(1.0))
        } else {
            self.cost_excl_vat()
        }
    }
}
