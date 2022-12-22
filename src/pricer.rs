use std::collections::HashMap;

use crate::{
    ocpi::{cdr::Cdr, tariff::TariffDimensionType},
    session::ChargePeriod,
    tariff::{PriceComponent, Tariffs},
};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::{session::ChargeSession, Result};
use rust_decimal::{prelude::Zero, Decimal};

pub struct Pricer {
    session: ChargeSession,
    tariffs: Tariffs,
}

impl Pricer {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Result<Self> {
        Ok(Self {
            session: ChargeSession::new(cdr, local_timezone)?,
            tariffs: Tariffs::new(&cdr.tariffs)?,
        })
    }

    pub fn build_report(&self) -> Result<Report> {
        let mut report = Report::new();

        for period in self.session.0.iter() {
            let tariff = self.tariffs.active_tariff(period)?;
            let mut dimensions = HashMap::new();

            for (dimension, component) in tariff.active_components(period)? {
                let volume = period.volume(dimension)?;
                dimensions.insert(dimension, DimensionReport::new(component, volume));
            }

            report
                .periods
                .push(PeriodReport::new(tariff.tariff_index, period, dimensions));
        }

        Ok(report)
    }
}

#[derive(Debug)]
pub struct Report {
    pub periods: Vec<PeriodReport>,
    pub time_costs: Decimal,
    pub parking_costs: Decimal,
    pub energy_costs: Decimal,
    pub flat_costs: Decimal,
}

impl Report {
    fn new() -> Self {
        Self {
            periods: Vec::new(),
            time_costs: Decimal::zero(),
            parking_costs: Decimal::zero(),
            energy_costs: Decimal::zero(),
            flat_costs: Decimal::zero(),
        }
    }
}

#[derive(Debug)]
pub struct PeriodReport {
    pub tariff_index: usize,
    pub start_date_time: DateTime<Utc>,
    pub end_date_time: DateTime<Utc>,
    pub dimensions: HashMap<TariffDimensionType, DimensionReport>,
}

impl PeriodReport {
    fn new(
        tariff_index: usize,
        period: &ChargePeriod,
        dimensions: HashMap<TariffDimensionType, DimensionReport>,
    ) -> Self {
        Self {
            tariff_index,
            start_date_time: period.start_instant.date_time,
            end_date_time: period.end_instant.date_time,
            dimensions,
        }
    }
}

#[derive(Debug)]
pub struct DimensionReport {
    pub price_component: PriceComponent,
    pub volume: Option<Decimal>,
    pub priced_volume: Option<Decimal>,
}

impl DimensionReport {
    pub fn new(price_component: PriceComponent, volume: Option<Decimal>) -> Self {
        Self {
            price_component,
            volume,
            priced_volume: volume,
        }
    }

    pub fn apply_step_size(&mut self) {}

    pub fn costs(&self) -> Decimal {
        let price = self.price_component.price;
        self.priced_volume.map_or(price, |v| v * price)
    }
}
