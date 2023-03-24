use std::collections::HashSet;

use chrono::{Duration, NaiveDate, NaiveTime, Timelike, Weekday};

use crate::ocpi::tariff::OcpiTariffRestriction;
use crate::session::{InstantData, PeriodData};
use crate::types::{Ampere, Kw, Kwh};

pub fn collect_restrictions(restriction: &OcpiTariffRestriction) -> Vec<Restriction> {
    let mut collected = Vec::new();

    match (restriction.start_time, restriction.end_time) {
        (Some(start_time), Some(end_time))
            if NaiveTime::from(end_time) < NaiveTime::from(start_time) =>
        {
            collected.push(Restriction::WrappingTime {
                start_time: start_time.into(),
                end_time: end_time.into(),
            })
        }
        (start_time, end_time) => {
            if let Some(start_time) = start_time {
                collected.push(Restriction::StartTime(start_time.into()))
            }

            if let Some(end_time) = end_time {
                collected.push(Restriction::EndTime(end_time.into()))
            }
        }
    }

    if let Some(start_date) = restriction.start_date {
        collected.push(Restriction::StartDate(start_date.into()))
    }

    if let Some(end_date) = restriction.end_date {
        collected.push(Restriction::EndDate(end_date.into()))
    }

    if let Some(min_kwh) = restriction.min_kwh {
        collected.push(Restriction::MinKwh(min_kwh))
    }

    if let Some(max_kwh) = restriction.max_kwh {
        collected.push(Restriction::MaxKwh(max_kwh))
    }

    if let Some(min_current) = restriction.min_current {
        collected.push(Restriction::MinCurrent(min_current))
    }

    if let Some(max_current) = restriction.max_current {
        collected.push(Restriction::MaxCurrent(max_current))
    }

    if let Some(min_power) = restriction.min_power {
        collected.push(Restriction::MinPower(min_power))
    }

    if let Some(max_power) = restriction.max_power {
        collected.push(Restriction::MaxPower(max_power))
    }

    if let Some(min_duration) = restriction.min_duration {
        collected.push(Restriction::MinDuration(min_duration.into()))
    }

    if let Some(max_duration) = restriction.max_duration {
        collected.push(Restriction::MaxDuration(max_duration.into()))
    }

    if !restriction.day_of_week.is_empty() {
        collected.push(Restriction::DayOfWeek(HashSet::from_iter(
            restriction.day_of_week.iter().copied().map(Into::into),
        )))
    }

    collected
}

#[derive(Debug, Clone)]
pub enum Restriction {
    StartTime(NaiveTime),
    EndTime(NaiveTime),
    WrappingTime {
        start_time: NaiveTime,
        end_time: NaiveTime,
    },
    StartDate(NaiveDate),
    EndDate(NaiveDate),
    MinKwh(Kwh),
    MaxKwh(Kwh),
    MinCurrent(Ampere),
    MaxCurrent(Ampere),
    MinPower(Kw),
    MaxPower(Kw),
    MinDuration(Duration),
    MaxDuration(Duration),
    DayOfWeek(HashSet<Weekday>),
    Reservation,
}

impl Restriction {
    /// Checks if this restriction is valid at `instant`. The time based restrictions are
    /// treated as exclusive comparisons.
    pub fn instant_validity_exclusive(&self, instant: &InstantData) -> bool {
        match self {
            &Self::WrappingTime {
                start_time,
                end_time,
            } => instant.local_time() >= start_time || instant.local_time() < end_time,
            &Self::StartTime(start_time) => instant.local_time() >= start_time,
            &Self::EndTime(end_time) => instant.local_time() < end_time,
            &Self::StartDate(start_date) => instant.local_date() >= start_date,
            &Self::EndDate(end_date) => instant.local_date() < end_date,
            &Self::MinKwh(min_energy) => instant.total_energy >= min_energy,
            &Self::MaxKwh(max_energy) => instant.total_energy < max_energy,
            &Self::MinDuration(min_duration) => instant.total_duration >= min_duration,
            &Self::MaxDuration(max_duration) => instant.total_duration < max_duration,
            Self::DayOfWeek(days) => days.contains(&instant.local_weekday()),
            _ => true,
        }
    }

    /// Checks if this restriction is valid at `instant`. The time based restriction are treated as
    /// inclusive comparisons.
    ///
    /// For example an instant at 00:00 on a tuesday is regarded as valid for a restriction that
    /// has a `DayOfWeek` which includes monday.
    pub fn instant_validity_inclusive(&self, instant: &InstantData) -> bool {
        match self {
            &Self::WrappingTime {
                start_time,
                end_time,
            } => instant.local_time() >= start_time || instant.local_time() < end_time,
            &Self::EndTime(end_time) => instant.local_time() <= end_time,
            &Self::EndDate(end_date) => {
                // Since the end date of this period is derived from the start date of the next period
                // we can't do a exclusive comparison. But we should still check that this period doesn't
                // end in the middle of the day on `end_date`.

                let is_before_end_date = instant.local_date() < end_date;
                let is_on_end_date = instant.local_date() == end_date;
                let is_at_midnight = instant.local_time().num_seconds_from_midnight() == 0;

                is_before_end_date || (is_on_end_date && is_at_midnight)
            }
            &Self::MinKwh(min_energy) => instant.total_energy >= min_energy,
            &Self::MaxKwh(max_energy) => instant.total_energy < max_energy,
            &Self::MinDuration(min_duration) => instant.total_duration >= min_duration,
            &Self::MaxDuration(max_duration) => instant.total_duration < max_duration,
            Self::DayOfWeek(days) => {
                let includes_weekday = days.contains(&instant.local_weekday());
                let includes_day_before = days.contains(&instant.local_weekday().pred());
                let is_at_midnight = instant.local_time().num_seconds_from_midnight() == 0;
                includes_weekday || (includes_day_before && is_at_midnight)
            }
            _ => true,
        }
    }

    /// Checks if this restriction is valid for `state`.
    pub fn period_validity(&self, state: &PeriodData) -> bool {
        match self {
            &Self::MinCurrent(min_current) => state
                .min_current
                .map(|current| current >= min_current)
                .unwrap_or(true),
            &Self::MaxCurrent(max_current) => state
                .max_current
                .map(|current| current < max_current)
                .unwrap_or(true),
            &Self::MinPower(min_power) => state
                .min_power
                .map(|power| power >= min_power)
                .unwrap_or(true),
            &Self::MaxPower(max_power) => state
                .max_power
                .map(|power| power < max_power)
                .unwrap_or(true),
            &Self::Reservation => todo!(),
            _ => true,
        }
    }
}
