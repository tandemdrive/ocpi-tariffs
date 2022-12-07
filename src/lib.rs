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

    fn is_active(&self, state: &ChargeState) -> Option<bool> {
        for restriction in self.restrictions.iter() {
            if !restriction.is_valid(state)? {
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
        let initial_state = ChargeState::new(local_timezone, cdr.start_date_time);
        let mut periods: Vec<ChargePeriod> = Vec::new();

        for (i, period) in cdr.charging_periods.iter().enumerate() {
            let last_state = periods
                .last()
                .map(|p| &p.end_state)
                .unwrap_or(&initial_state);

            let end_date_time = cdr
                .charging_periods
                .get(i + 1)
                .map(|p| p.start_date_time)
                .unwrap_or(cdr.stop_date_time);

            let start_state = last_state.next_start(period);
            let end_state = last_state.next_end(period, end_date_time);

            periods.push(ChargePeriod {
                start_state,
                end_state,
            });
        }

        Self { periods }
    }
}

pub struct ChargePeriod {
    start_state: ChargeState,
    end_state: ChargeState,
}

impl ChargePeriod {
    pub fn new(start_state: ChargeState, end_state: ChargeState) -> Self {
        Self {
            start_state,
            end_state,
        }
    }
}

#[derive(Clone)]
pub struct ChargeState {
    local_timezone: Tz,
    date_time: DateTime,
    total_duration: Option<Duration>,
    total_energy: Option<Number>,
    max_current: Option<Number>,
    min_current: Option<Number>,
    max_power: Option<Number>,
    min_power: Option<Number>,
}

impl ChargeState {
    fn new(local_timezone: Tz, date_time: DateTime) -> Self {
        Self {
            local_timezone,
            date_time,
            total_duration: Some(Duration::zero()),
            total_energy: Some(Number::zero()),
            max_current: None,
            min_current: None,
            max_power: None,
            min_power: None,
        }
    }

    fn next_start(&self, period: &OcpiChargingPeriod) -> Self {
        let mut next = self.clone();

        next.min_power = None;
        next.max_power = None;
        next.min_current = None;
        next.max_current = None;

        for dimension in period.dimensions.iter() {
            match dimension.dimension_type {
                OcpiCdrDimensionType::MinCurrent => next.min_current = Some(dimension.volume),
                OcpiCdrDimensionType::MaxCurrent => next.max_current = Some(dimension.volume),
                OcpiCdrDimensionType::MaxPower => next.max_power = Some(dimension.volume),
                OcpiCdrDimensionType::MinPower => next.min_power = Some(dimension.volume),
                _ => {}
            }
        }

        next
    }

    fn next_end(&self, period: &OcpiChargingPeriod, date_time: DateTime) -> Self {
        let mut next = self.clone();
        next.date_time = date_time;

        for dimension in period.dimensions.iter() {
            match dimension.dimension_type {
                OcpiCdrDimensionType::Time => {
                    next.total_duration = next.total_duration.map(|duration| {
                        let millis = dimension.volume * Decimal::from_str("3600_000").unwrap();
                        Duration::milliseconds(millis.try_into().unwrap()) + duration
                    });
                }
                OcpiCdrDimensionType::Energy => {
                    next.total_energy = next.total_energy.map(|energy| energy + dimension.volume)
                }
                _ => {}
            }
        }

        next
    }

    fn local_time(&self) -> NaiveTime {
        self.date_time.with_timezone(&self.local_timezone).time()
    }

    fn local_date(&self) -> NaiveDate {
        self.date_time
            .with_timezone(&self.local_timezone)
            .date_naive()
    }

    fn local_weekday(&self) -> Weekday {
        self.date_time.with_timezone(&self.local_timezone).weekday()
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
