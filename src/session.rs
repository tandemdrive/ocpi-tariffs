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

/// Describes the properties of a single charging period.
#[derive(Debug)]
pub struct ChargePeriod {
    /// Holds properties that are valid for the entirety of this period.
    pub period_data: PeriodData,
    /// Holds properties that are valid at start instant of this period.
    pub start_instant: InstantData,
    /// Holds properties that are valid at the end instant of this period.
    pub end_instant: InstantData,
}

impl ChargePeriod {
    /// Construct a new `ChargePeriod` with zeroed values. Should be the first period in the
    /// session.
    fn new(
        local_timezone: Tz,
        period: &OcpiChargingPeriod,
        end_date_time: DateTime<Utc>,
    ) -> Result<Self> {
        let charge_state = PeriodData::new(period);
        let start_instant = InstantData::zero(period.start_date_time, local_timezone);
        let end_instant = start_instant.next(&charge_state, end_date_time)?;

        Ok(Self {
            period_data: charge_state,
            start_instant,
            end_instant,
        })
    }

    /// Construct a period with the properties of `period` that ends on `end_date_time` which succeeds `self`.
    fn next(&self, period: &OcpiChargingPeriod, end_date_time: DateTime<Utc>) -> Result<Self> {
        let charge_state = PeriodData::new(period);
        let start_instant = self.end_instant;
        let end_instant = start_instant.next(&charge_state, end_date_time)?;

        Ok(Self {
            period_data: charge_state,
            start_instant,
            end_instant,
        })
    }

    /// Extract the volume of a certain dimension. If might be that the CDR did not provide a
    /// volume for `dimension` in this period in which case the error `MissingPricingDimension` is returned.
    ///
    /// The dimension `Flat` never has a volume in which case `None` is returned.
    pub fn volume(&self, dimension: TariffDimensionType) -> Result<Option<Decimal>> {
        let volume = match dimension {
            TariffDimensionType::Flat => None,
            TariffDimensionType::Time => Some(
                self.period_data
                    .duration
                    .ok_or(Error::MissingPricingDimension)?,
            ),
            TariffDimensionType::Energy => Some(
                self.period_data
                    .energy
                    .ok_or(Error::MissingPricingDimension)?,
            ),
            TariffDimensionType::ParkingTime => todo!(),
        };

        Ok(volume)
    }
}

/// This describes the properties in the charge session that a valid during a certain period. For
/// example the `duration` field is the charge duration during a certain charging period.
#[derive(Debug)]
pub struct PeriodData {
    pub max_current: Option<Decimal>,
    pub min_current: Option<Decimal>,
    pub max_power: Option<Decimal>,
    pub min_power: Option<Decimal>,
    pub duration: Option<Decimal>,
    pub energy: Option<Decimal>,
}

/// This describes the properties in the charge session that are instantaneous. For example
/// the `total_energy` is the total amount of energy in the charge session at a certain instant.
#[derive(Debug, Clone, Copy)]
pub struct InstantData {
    local_timezone: Tz,
    pub date_time: DateTime<Utc>,
    pub total_duration: Duration,
    pub total_energy: Decimal,
}

impl InstantData {
    fn zero(date_time: DateTime<Utc>, local_timezone: Tz) -> Self {
        Self {
            date_time,
            local_timezone,
            total_duration: Duration::zero(),
            total_energy: Decimal::zero(),
        }
    }

    fn next(&self, state: &PeriodData, date_time: DateTime<Utc>) -> Result<Self> {
        let mut next = self.clone();

        next.date_time = date_time;

        if let Some(duration) = state.duration {
            let millis: i64 = (duration * Decimal::from_str("3600_000").unwrap())
                .try_into()
                .map_err(|_| Error::DurationOverflow)?;

            next.total_duration = self.total_duration + Duration::milliseconds(millis);
        }

        if let Some(energy) = state.energy {
            next.total_energy = next.total_energy + energy;
        }

        Ok(next)
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

impl PeriodData {
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
