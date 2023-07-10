use crate::{
    ocpi::cdr::{Cdr, OcpiCdrDimension, OcpiChargingPeriod},
    types::{
        electricity::{Ampere, Kw, Kwh},
        time::DateTime,
    },
};

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use chrono_tz::Tz;

pub struct ChargeSession {
    pub start_date_time: DateTime,
    pub periods: Vec<ChargePeriod>,
}

impl ChargeSession {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Self {
        let mut periods: Vec<ChargePeriod> = Vec::new();

        for (i, period) in cdr.charging_periods.iter().enumerate() {
            let end_date_time = if let Some(next_period) = cdr.charging_periods.get(i + 1) {
                next_period.start_date_time
            } else {
                cdr.end_date_time
            };

            let next = if let Some(last) = periods.last() {
                last.next(period, end_date_time)
            } else {
                ChargePeriod::new(local_timezone, period, end_date_time)
            };

            periods.push(next);
        }

        Self {
            periods,
            start_date_time: cdr.start_date_time,
        }
    }
}

/// Describes the properties of a single charging period.
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
    fn new(local_timezone: Tz, period: &OcpiChargingPeriod, end_date_time: DateTime) -> Self {
        let charge_state = PeriodData::new(period);
        let start_instant = InstantData::zero(period.start_date_time, local_timezone);
        let end_instant = start_instant.next(&charge_state, end_date_time);

        Self {
            period_data: charge_state,
            start_instant,
            end_instant,
        }
    }

    /// Construct a period with the properties of `period` that ends on `end_date_time` which succeeds `self`.
    fn next(&self, period: &OcpiChargingPeriod, end_date_time: DateTime) -> Self {
        let charge_state = PeriodData::new(period);
        let start_instant = self.end_instant.clone();
        let end_instant = start_instant.next(&charge_state, end_date_time);

        Self {
            period_data: charge_state,
            start_instant,
            end_instant,
        }
    }
}

/// This describes the properties in the charge session that a valid during a certain period. For
/// example the `duration` field is the charge duration during a certain charging period.
pub struct PeriodData {
    pub max_current: Option<Ampere>,
    pub min_current: Option<Ampere>,
    pub max_power: Option<Kw>,
    pub min_power: Option<Kw>,
    pub charging_duration: Option<Duration>,
    pub parking_duration: Option<Duration>,
    pub reservation_duration: Option<Duration>,
    pub energy: Option<Kwh>,
}

/// This describes the properties in the charge session that are instantaneous. For example
/// the `total_energy` is the total amount of energy in the charge session at a certain instant.
#[derive(Clone)]
pub struct InstantData {
    local_timezone: Tz,
    pub date_time: DateTime,
    pub total_charging_duration: Duration,
    pub total_duration: Duration,
    pub total_energy: Kwh,
}

impl InstantData {
    fn zero(date_time: DateTime, local_timezone: Tz) -> Self {
        Self {
            date_time,
            local_timezone,
            total_charging_duration: Duration::zero(),
            total_duration: Duration::zero(),
            total_energy: Kwh::zero(),
        }
    }

    fn next(&self, state: &PeriodData, date_time: DateTime) -> Self {
        let mut next = self.clone();

        let duration = date_time.signed_duration_since(next.date_time);

        next.total_duration = next
            .total_duration
            .checked_add(&duration)
            .unwrap_or_else(Duration::max_value);

        next.date_time = date_time;

        if let Some(duration) = state.charging_duration {
            next.total_charging_duration = next
                .total_charging_duration
                .checked_add(&duration)
                .unwrap_or_else(Duration::max_value);
        }

        if let Some(energy) = state.energy {
            next.total_energy = next.total_energy.saturating_add(energy);
        }

        next
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
            parking_duration: None,
            reservation_duration: None,
            max_current: None,
            min_current: None,
            max_power: None,
            min_power: None,
            charging_duration: None,
            energy: None,
        };

        for dimension in period.dimensions.iter() {
            match *dimension {
                OcpiCdrDimension::MinCurrent(volume) => inst.min_current = Some(volume),
                OcpiCdrDimension::MaxCurrent(volume) => inst.max_current = Some(volume),
                OcpiCdrDimension::MaxPower(volume) => inst.max_power = Some(volume),
                OcpiCdrDimension::MinPower(volume) => inst.min_power = Some(volume),
                OcpiCdrDimension::Energy(volume) => inst.energy = Some(volume),
                OcpiCdrDimension::Time(volume) => {
                    inst.charging_duration = Some(volume.into());
                }
                OcpiCdrDimension::ParkingTime(volume) => {
                    inst.parking_duration = Some(volume.into());
                }
                OcpiCdrDimension::ReservationTime(volume) => {
                    inst.reservation_duration = Some(volume.into());
                }
            }
        }

        inst
    }
}
