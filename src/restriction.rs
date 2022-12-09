use std::collections::HashSet;

use chrono::{Duration, NaiveDate, NaiveTime, Timelike, Weekday};

use crate::ocpi::tariff::OcpiTariffRestriction;
use crate::ocpi::Number;
use crate::{ChargeInstant, ChargeState, Error};

pub fn collect_restrictions(
    restriction: &OcpiTariffRestriction,
) -> Result<Vec<Restriction>, Error> {
    let mut collected = Vec::new();

    match (&restriction.start_time, &restriction.end_time) {
        (Some(start_time), Some(end_time)) if end_time < start_time => {
            collected.push(Restriction::WrappingTime {
                start_time: start_time.parse()?,
                end_time: end_time.parse()?,
            })
        }
        (start_time, end_time) => {
            if let Some(start_time) = start_time {
                collected.push(Restriction::StartTime(start_time.parse()?))
            }

            if let Some(end_time) = end_time {
                collected.push(Restriction::EndTime(end_time.parse()?))
            }
        }
    }

    if let Some(start_date) = &restriction.start_date {
        collected.push(Restriction::StartDate(start_date.parse()?))
    }

    if let Some(end_date) = &restriction.end_date {
        collected.push(Restriction::EndDate(end_date.parse()?))
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
        collected.push(Restriction::MinDuration(Duration::seconds(min_duration)))
    }

    if let Some(max_duration) = restriction.max_duration {
        collected.push(Restriction::MaxDuration(Duration::seconds(max_duration)))
    }

    if !restriction.day_of_week.is_empty() {
        collected.push(Restriction::DayOfWeek(HashSet::from_iter(
            restriction.day_of_week.iter().copied().map(Into::into),
        )))
    }

    Ok(collected)
}

#[derive(Debug)]
pub enum Restriction {
    StartTime(NaiveTime),
    EndTime(NaiveTime),
    WrappingTime {
        start_time: NaiveTime,
        end_time: NaiveTime,
    },
    StartDate(NaiveDate),
    EndDate(NaiveDate),
    MinKwh(Number),
    MaxKwh(Number),
    MinCurrent(Number),
    MaxCurrent(Number),
    MinPower(Number),
    MaxPower(Number),
    MinDuration(Duration),
    MaxDuration(Duration),
    DayOfWeek(HashSet<Weekday>),
    Reservation,
}

impl Restriction {

    /// Checks if this restriction is valid at `instant`. The time based restrictions are
    /// treated as exclusive comparisons.
    pub fn instant_validity_exclusive(&self, instant: &ChargeInstant) -> Option<bool> {
        match self {
            &Self::WrappingTime {
                start_time,
                end_time,
            } => Some(instant.local_time() >= start_time || instant.local_time() < end_time),
            &Self::StartTime(start_time) => Some(instant.local_time() >= start_time),
            &Self::EndTime(end_time) => Some(instant.local_time() < end_time),
            &Self::StartDate(start_date) => Some(instant.local_date() >= start_date),
            &Self::EndDate(end_date) => Some(instant.local_date() < end_date),
            &Self::MinKwh(min_energy) => instant.energy.map(|energy| energy >= min_energy),
            &Self::MaxKwh(max_energy) => instant.energy.map(|energy| energy < max_energy),
            &Self::MinDuration(min_duration) => {
                instant.duration.map(|duration| duration >= min_duration)
            }
            &Self::MaxDuration(max_duration) => {
                instant.duration.map(|duration| duration < max_duration)
            }
            Self::DayOfWeek(days) => Some(days.contains(&instant.local_weekday())),
            _ => Some(true),
        }
    }

    /// Checks if this restriction is valid for `state`.
    pub fn state_validity(&self, state: &ChargeState) -> Option<bool> {
        match self {
            &Self::MinCurrent(min_current) => {
                state.min_current.map(|current| current >= min_current)
            }
            &Self::MaxCurrent(max_current) => {
                state.max_current.map(|current| current < max_current)
            }
            &Self::MinPower(min_power) => state.min_power.map(|power| power >= min_power),
            &Self::MaxPower(max_power) => state.max_power.map(|power| power < max_power),
            &Self::Reservation => todo!(),
            _ => Some(true),
        }
    }

    /// Checks if this restriction is valid at `instant`. The time based restriction are treated as
    /// inclusive comparisons.
    ///
    /// For example an instant at 00:00 on a tuesday is regarded as valid for a restriction that
    /// has a `DayOfWeek` which includes monday.
    pub fn instant_validity_inclusive(&self, instant: &ChargeInstant) -> Option<bool> {
        match self {
            &Self::WrappingTime {
                start_time,
                end_time,
            } => Some(instant.local_time() >= start_time || instant.local_time() < end_time),
            &Self::EndTime(end_time) => Some(instant.local_time() <= end_time),
            &Self::EndDate(end_date) => {
                // Since the end date of this period is derived from the start date of the next period
                // we can't do a exclusive comparison. But we should still check that this period doesnt
                // end in the middle of the day on `end_date`.

                let is_before_end_date = instant.local_date() < end_date;
                let is_on_end_date = instant.local_date() == end_date;
                let is_at_midnight = instant.local_time().num_seconds_from_midnight() == 0;

                Some(is_before_end_date || (is_on_end_date && is_at_midnight))
            }
            &Self::MinKwh(min_energy) => instant.energy.map(|energy| energy >= min_energy),
            &Self::MaxKwh(max_energy) => instant.energy.map(|energy| energy < max_energy),
            &Self::MinDuration(min_duration) => {
                instant.duration.map(|duration| duration >= min_duration)
            }
            &Self::MaxDuration(max_duration) => {
                instant.duration.map(|duration| duration < max_duration)
            }
            Self::DayOfWeek(days) => {
                let includes_weekday = days.contains(&instant.local_weekday());
                let includes_day_before = days.contains(&instant.local_weekday().pred());
                let is_at_midnight = instant.local_time().num_seconds_from_midnight() == 0;

                Some(includes_weekday || (includes_day_before && is_at_midnight))
            }
            _ => Some(true),
        }
    }
}
