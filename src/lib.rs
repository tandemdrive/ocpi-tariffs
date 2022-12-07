mod ocpi;
mod restriction;

use std::str::FromStr;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use chrono_tz::Tz;
use ocpi::{
    cdr::{Cdr, OcpiCdrDimensionType, OcpiChargingPeriod},
    tariff::{OcpiPriceComponent, OcpiTariffElement, TariffDimensionType},
    DateTime, Number,
};

use restriction::{collect_restrictions, Restriction};
use rust_decimal::{prelude::Zero, Decimal};

/// An event is a point in time where a price component for a certain dimension either becomes
/// active or inactive
pub struct Event<'t> {
    /// The time at which this event occured
    pub time: DateTime,
    /// Whether this event activates or deactivates the price component.
    pub kind: EventKind,
    /// The dimension for which this event applies.
    pub dimension: TariffDimensionType,
    /// The `TariffElement` for which this event applies.
    pub element: &'t OcpiTariffElement,
}

#[derive(Debug)]
struct PriceComponent {
    /// Price per unit (excluding VAT) for this tariff dimension
    pub price: Number,

    pub vat: Option<Number>,

    /// Minimum amount to be billed. This unit will be billed in this step_size
    /// blocks. For example: if type is time and step_size is 300, then time will
    /// be billed in blocks of 5 minutes, so if 6 minutes is used, 10 minutes (2
    /// blocks of step_size) will be billed
    pub step_size: u64,
}

impl From<&OcpiPriceComponent> for PriceComponent {
    fn from(component: &OcpiPriceComponent) -> Self {
        Self {
            price: component.price,
            vat: component.vat,
            step_size: component.step_size,
        }
    }
}

#[derive(Debug)]
struct TariffElement {
    restrictions: Vec<Restriction>,
    time_component: Option<PriceComponent>,
    parking_component: Option<PriceComponent>,
    flat_component: Option<PriceComponent>,
    energy_component: Option<PriceComponent>,
}

impl TariffElement {
    fn new(ocpi_element: &OcpiTariffElement) -> Result<Self, Error> {
        let restrictions = if let Some(restrictions) = &ocpi_element.restrictions {
            collect_restrictions(restrictions)?
        } else {
            Vec::new()
        };

        let mut element = Self {
            restrictions,
            time_component: None,
            parking_component: None,
            flat_component: None,
            energy_component: None,
        };

        for ocpi_component in ocpi_element.price_components.iter() {
            (match ocpi_component.component_type {
                TariffDimensionType::Time => &mut element.time_component,
                TariffDimensionType::ParkingTime => &mut element.parking_component,
                TariffDimensionType::Energy => &mut element.energy_component,
                TariffDimensionType::Flat => &mut element.flat_component,
            })
            .get_or_insert(ocpi_component.into());
        }

        Ok(element)
    }

    fn is_active(&self, period: &ChargePeriod) -> Option<bool> {
        for restriction in self.restrictions.iter() {
            if !restriction.is_valid(period)? {
                return Some(false);
            }
        }

        Some(true)
    }
}

pub struct ChargeSession {
    periods: Vec<ChargePeriod>,
}

impl ChargeSession {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Self {
        let mut periods: Vec<ChargePeriod> = Vec::new();

        for (i, period) in cdr.charging_periods.iter().enumerate() {
            let end_date_time = if let Some(next_period) = cdr.charging_periods.get(i + 1) {
                next_period.start_date_time
            } else {
                cdr.stop_date_time
            };

            let next = if let Some(last) = periods.last() {
                last.next(period, end_date_time)
            } else {
                ChargePeriod::new(local_timezone, period, end_date_time)
            };

            periods.push(next);
        }

        Self { periods }
    }
}

pub struct ChargePeriod {
    local_timezone: Tz,
    start_date_time: DateTime,
    end_date_time: DateTime,
    state: ChargeState,
    start_aggregate: ChargeAggregate,
    end_aggregate: ChargeAggregate,
}

impl ChargePeriod {
    pub fn new(local_timezone: Tz, period: &OcpiChargingPeriod, end_date_time: DateTime) -> Self {
        let state = ChargeState::new(period);
        let start_aggregate = ChargeAggregate::zero();
        let end_aggregate = start_aggregate.add(period);

        Self {
            local_timezone,
            end_date_time,
            start_date_time: period.start_date_time,
            state,
            start_aggregate,
            end_aggregate,
        }
    }

    pub fn next(&self, period: &OcpiChargingPeriod, end_date_time: DateTime) -> Self {
        let state = ChargeState::new(period);
        let start_aggregate = self.end_aggregate;
        let end_aggregate = start_aggregate.add(period);

        Self {
            local_timezone: self.local_timezone,
            start_date_time: period.start_date_time,
            end_date_time,
            state,
            start_aggregate,
            end_aggregate,
        }
    }

    fn local_start_time(&self) -> NaiveTime {
        self.start_date_time
            .with_timezone(&self.local_timezone)
            .time()
    }

    fn local_start_date(&self) -> NaiveDate {
        self.start_date_time
            .with_timezone(&self.local_timezone)
            .date_naive()
    }

    fn local_start_weekday(&self) -> Weekday {
        self.start_date_time
            .with_timezone(&self.local_timezone)
            .weekday()
    }
}

#[derive(Debug)]
pub struct ChargeState {
    max_current: Option<Number>,
    min_current: Option<Number>,
    max_power: Option<Number>,
    min_power: Option<Number>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChargeAggregate {
    duration: Option<Duration>,
    energy: Option<Number>,
}

impl ChargeAggregate {
    pub fn zero() -> Self {
        Self {
            duration: Some(Duration::zero()),
            energy: Some(Number::zero()),
        }
    }

    pub fn new() -> Self {
        Self {
            duration: None,
            energy: None,
        }
    }

    pub fn add(&self, period: &OcpiChargingPeriod) -> Self {
        let mut result = Self::new();

        for dimension in period.dimensions.iter() {
            match dimension.dimension_type {
                OcpiCdrDimensionType::Time => {
                    result.duration = self.duration.map(|duration| {
                        let millis = dimension.volume * Decimal::from_str("3600_000").unwrap();
                        Duration::milliseconds(millis.try_into().unwrap()) + duration
                    });
                }
                OcpiCdrDimensionType::Energy => {
                    result.energy = self.energy.map(|energy| energy + dimension.volume)
                }
                _ => {}
            }
        }

        result
    }
}

impl ChargeState {

    fn new(period: &OcpiChargingPeriod) -> Self {
        let mut inst = Self {
            max_current: None,
            min_current: None,
            max_power: None,
            min_power: None,
        };

        for dimension in period.dimensions.iter() {
            match dimension.dimension_type {
                OcpiCdrDimensionType::MinCurrent => inst.min_current = Some(dimension.volume),
                OcpiCdrDimensionType::MaxCurrent => inst.max_current = Some(dimension.volume),
                OcpiCdrDimensionType::MaxPower => inst.max_power = Some(dimension.volume),
                OcpiCdrDimensionType::MinPower => inst.min_power = Some(dimension.volume),
                _ => {}
            }
        }

        inst
    }
}

pub enum EventKind {
    Activated,
    Deactivated(Vec<Restriction>),
}

#[derive(Debug)]
pub enum Error {
    InvalidTimeZone(chrono_tz::ParseError),
    InvalidDateTime(chrono::ParseError),
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
