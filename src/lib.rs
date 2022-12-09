mod ocpi;
mod restriction;

use std::str::FromStr;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use chrono_tz::Tz;
use ocpi::{
    cdr::{Cdr, OcpiCdrDimensionType, OcpiChargingPeriod},
    tariff::{OcpiPriceComponent, OcpiTariff, OcpiTariffElement, TariffDimensionType},
    DateTime, Number,
};

use restriction::{collect_restrictions, Restriction};
use rust_decimal::{prelude::Zero, Decimal};

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

struct Tariff {
    elements: Vec<TariffElement>,
    start_date_time: Option<DateTime>,
    end_date_time: Option<DateTime>,
}

impl Tariff {
    pub fn new(tariff: &OcpiTariff) -> Result<Self, Error> {
        Ok(Self {
            start_date_time: tariff.start_date_time,
            end_date_time: tariff.end_date_time,
            elements: tariff
                .elements
                .iter()
                .map(TariffElement::new)
                .collect::<Result<_, Error>>()?,
        })
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
}

struct ChargeSession {
    periods: Vec<ChargePeriod>,
    tariffs: Vec<Tariff>,
}

impl ChargeSession {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Result<Self, Error> {
        let tariffs = cdr
            .tariffs
            .iter()
            .map(Tariff::new)
            .collect::<Result<_, _>>()?;

        let mut periods: Vec<ChargePeriod> = Vec::new();

        for (i, period) in cdr.charging_periods.iter().enumerate() {
            let end_date_time = if let Some(next_period) = cdr.charging_periods.get(i + 1) {
                next_period.start_date_time
            } else {
                cdr.stop_date_time
            };

            let next = if let Some(last) = periods.last() {
                last.next(period, end_date_time)?
            } else {
                ChargePeriod::new(local_timezone, period, end_date_time)?
            };

            periods.push(next);
        }

        Ok(Self { periods, tariffs })
    }
}

#[derive(Debug)]
pub struct ChargePeriod {
    charge_state: ChargeState,
    start_instant: ChargeInstant,
    end_instant: ChargeInstant,
}

impl ChargePeriod {
    fn new(
        local_timezone: Tz,
        period: &OcpiChargingPeriod,
        end_date_time: DateTime,
    ) -> Result<Self, Error> {
        let charge_state = ChargeState::new(period);
        let start_instant = ChargeInstant::zero(period.start_date_time, local_timezone);
        let end_instant = start_instant.next(period, end_date_time)?;

        Ok(Self {
            charge_state,
            start_instant,
            end_instant,
        })
    }

    fn next(&self, period: &OcpiChargingPeriod, end_date_time: DateTime) -> Result<Self, Error> {
        let charge_state = ChargeState::new(period);
        let start_instant = self.end_instant;
        let end_instant = start_instant.next(period, end_date_time)?;

        Ok(Self {
            charge_state,
            start_instant,
            end_instant,
        })
    }

    fn is_tariff_active(&self, tariff: &Tariff) -> bool {
        if let Some(start_date_time) = tariff.start_date_time {
            if self.start_instant.date_time < start_date_time {
                return false;
            }
        }

        if let Some(end_date_time) = tariff.end_date_time {
            if self.start_instant.date_time > end_date_time {
                return false;
            }
        }

        true
    }

    fn is_tariff_element_active(&self, element: TariffElement) -> Option<bool> {
        for restriction in element.restrictions {
            if !restriction.instant_validity_exclusive(&self.start_instant)? {
                return Some(false);
            }

            if !restriction.state_validity(&self.charge_state)? {
                return Some(false);
            }
        }

        Some(true)
    }

    fn is_tariff_element_active_at_end(&self, element: TariffElement) -> Option<bool> {
        for restriction in element.restrictions {
            if !restriction.instant_validity_inclusive(&self.end_instant)? {
                return Some(false);
            }
        }

        Some(true)
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
pub struct ChargeInstant {
    local_timezone: Tz,
    date_time: DateTime,
    duration: Option<Duration>,
    energy: Option<Number>,
}

impl ChargeInstant {

    fn zero(date_time: DateTime, local_timezone: Tz) -> Self {
        Self {
            date_time,
            local_timezone,
            duration: Some(Duration::zero()),
            energy: Some(Number::zero()),
        }
    }

    fn new(date_time: DateTime, local_timezone: Tz) -> Self {
        Self {
            date_time,
            local_timezone,
            duration: None,
            energy: None,
        }
    }

    fn next(&self, period: &OcpiChargingPeriod, date_time: DateTime) -> Result<Self, Error> {
        let mut result = Self::new(date_time, self.local_timezone);

        for dimension in period.dimensions.iter() {
            match dimension.dimension_type {
                OcpiCdrDimensionType::Time => {
                    let millis: i64 = (dimension.volume * Decimal::from_str("3600_000").unwrap())
                        .try_into()
                        .map_err(|_| Error::DurationOverflow)?;

                    result.duration = self
                        .duration
                        .map(|duration| Duration::milliseconds(millis) + duration)
                }
                OcpiCdrDimensionType::Energy => {
                    result.energy = self.energy.map(|energy| energy + dimension.volume)
                }
                _ => {}
            }
        }

        Ok(result)
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

#[derive(Debug)]
pub enum Error {
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
