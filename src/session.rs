use std::str::FromStr;

use crate::ocpi::{
    cdr::{Cdr, OcpiCdrDimensionType, OcpiChargingPeriod},
    tariff::TariffDimensionType,
};
use crate::{Error, Result};

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;

use rust_decimal::{prelude::Zero, Decimal};

pub struct ChargeSession(pub Vec<ChargePeriod>);

impl ChargeSession {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Result<Self> {
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

        Ok(Self(periods))
    }
}

#[derive(Debug)]
pub struct ChargePeriod {
    pub charge_state: ChargeState,
    pub start_instant: ChargeInstant,
    pub end_instant: ChargeInstant,
}

impl ChargePeriod {
    fn new(
        local_timezone: Tz,
        period: &OcpiChargingPeriod,
        end_date_time: DateTime<Utc>,
    ) -> Result<Self> {
        let charge_state = ChargeState::new(period);
        let start_instant = ChargeInstant::zero(period.start_date_time, local_timezone);
        let end_instant = start_instant.next(&charge_state, end_date_time)?;

        Ok(Self {
            charge_state,
            start_instant,
            end_instant,
        })
    }

    fn next(&self, period: &OcpiChargingPeriod, end_date_time: DateTime<Utc>) -> Result<Self> {
        let charge_state = ChargeState::new(period);
        let start_instant = self.end_instant;
        let end_instant = start_instant.next(&charge_state, end_date_time)?;

        Ok(Self {
            charge_state,
            start_instant,
            end_instant,
        })
    }

    pub fn volume(&self, dimension: TariffDimensionType) -> Result<Option<Decimal>> {
        let volume = match dimension {
            TariffDimensionType::Flat => None,
            TariffDimensionType::Time => Some(
                self.charge_state
                    .duration
                    .ok_or(Error::MissingPricingDimension)?,
            ),
            TariffDimensionType::Energy => Some(
                self.charge_state
                    .energy
                    .ok_or(Error::MissingPricingDimension)?,
            ),
            TariffDimensionType::ParkingTime => todo!(),
        };

        Ok(volume)
    }
}

#[derive(Debug)]
pub struct ChargeState {
    pub max_current: Option<Decimal>,
    pub min_current: Option<Decimal>,
    pub max_power: Option<Decimal>,
    pub min_power: Option<Decimal>,
    pub duration: Option<Decimal>,
    pub energy: Option<Decimal>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChargeInstant {
    local_timezone: Tz,
    pub date_time: DateTime<Utc>,
    pub total_duration: Option<Duration>,
    pub total_energy: Option<Decimal>,
}

impl ChargeInstant {
    fn zero(date_time: DateTime<Utc>, local_timezone: Tz) -> Self {
        Self {
            date_time,
            local_timezone,
            total_duration: Some(Duration::zero()),
            total_energy: Some(Decimal::zero()),
        }
    }

    fn new(date_time: DateTime<Utc>, local_timezone: Tz) -> Self {
        Self {
            date_time,
            local_timezone,
            total_duration: None,
            total_energy: None,
        }
    }

    fn next(&self, state: &ChargeState, date_time: DateTime<Utc>) -> Result<Self> {
        let mut result = Self::new(date_time, self.local_timezone);

        if let Some(duration) = state.duration {
            let millis: i64 = (duration * Decimal::from_str("3600_000").unwrap())
                .try_into()
                .map_err(|_| Error::DurationOverflow)?;

            result.total_duration = self
                .total_duration
                .map(|total_duration| Duration::milliseconds(millis) + total_duration)
        }

        if let Some(energy) = state.energy {
            result.total_energy = self.total_energy.map(|total_energy| total_energy + energy)
        }

        Ok(result)
    }

    pub fn local_time(&self) -> NaiveTime {
        self.date_time.with_timezone(&self.local_timezone).time()
    }

    pub fn local_date(&self) -> NaiveDate {
        self.date_time
            .with_timezone(&self.local_timezone)
            .date_naive()
    }

    pub fn local_weekday(&self) -> Weekday {
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
            duration: None,
            energy: None,
        };

        for dimension in period.dimensions.iter() {
            match dimension.dimension_type {
                OcpiCdrDimensionType::MinCurrent => inst.min_current = Some(dimension.volume),
                OcpiCdrDimensionType::MaxCurrent => inst.max_current = Some(dimension.volume),
                OcpiCdrDimensionType::MaxPower => inst.max_power = Some(dimension.volume),
                OcpiCdrDimensionType::MinPower => inst.min_power = Some(dimension.volume),
                OcpiCdrDimensionType::Time => inst.duration = Some(dimension.volume),
                OcpiCdrDimensionType::Energy => inst.energy = Some(dimension.volume),
                _ => {}
            }
        }

        inst
    }
}
