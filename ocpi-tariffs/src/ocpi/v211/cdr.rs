use serde::{Deserialize, Serialize};

use super::tariff::OcpiTariff;
use crate::{
    null_default,
    ocpi::v221,
    types::{
        electricity::{Ampere, Kwh},
        money::{Money, Price},
        time::{DateTime, HoursDecimal},
    },
};

/// The CDR object describes the Charging Session and its costs. How these costs are build up etc.
#[derive(Clone, Deserialize, Serialize)]
pub struct Cdr {
    /// Start timestamp of the charging session.
    pub start_date_time: DateTime,

    /// Stop timestamp of the charging session.
    pub stop_date_time: DateTime,

    /// Currency of the CDR in ISO 4217 Code.
    pub currency: String,

    /// List of relevant tariff elements.
    #[serde(deserialize_with = "null_default", default)]
    pub tariffs: Vec<OcpiTariff>,

    /// Describes the location that the charge-session took place at.
    pub location: OcpiLocation,

    /// List of charging periods that make up this charging session> A session should consist of 1 or
    /// more periods, where each period has a different relevant Tariff.
    pub charging_periods: Vec<OcpiChargingPeriod>,

    /// Total cost of this transaction.
    pub total_cost: Money,

    /// Total energy charged, in kWh.
    pub total_energy: Kwh,

    /// Total time charging, in hours
    pub total_time: HoursDecimal,

    /// Total time not charging, in hours
    pub total_parking_time: Option<HoursDecimal>,

    /// Timestamp when this CDR was last updated
    pub last_updated: DateTime,
}

/// Describes the location that the charge-session took place at.
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiLocation {
    /// ISO 3166-1 alpha-3 code for the country of this location.
    pub country: String,
    /// One of IANA tzdata's TZ-values representing the time zone of the location. Examples: "Europe/Oslo", "Europe/Zurich"
    pub time_zone: Option<String>,
}

/// The volume that has been consumed for a specific dimension during a charging period.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type", content = "volume")]
pub enum OcpiCdrDimension {
    /// Consumed energy in `kWh`.
    Energy(Kwh),
    /// Flat fee, no unit.
    Flat(()),
    /// The peak current, in 'A', during this period.
    MaxCurrent(Ampere),
    /// The lowest current, in `A`, during this period.
    MinCurrent(Ampere),
    /// The parking time, in hours, consumed in this period.
    ParkingTime(HoursDecimal),
    /// The charging time, in hours, consumed in this period.
    Time(HoursDecimal),
}

/// A single charging period, containing a non empty list of charge dimensions.
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiChargingPeriod {
    /// Start timestamp of the charging period. This period ends when a next period starts, the
    /// last period ends when the session ends
    pub start_date_time: DateTime,

    /// List of relevant values for this charging period
    pub dimensions: Vec<OcpiCdrDimension>,
}

impl From<OcpiChargingPeriod> for v221::cdr::OcpiChargingPeriod {
    fn from(period: OcpiChargingPeriod) -> Self {
        Self {
            start_date_time: period.start_date_time,
            dimensions: period
                .dimensions
                .into_iter()
                .filter_map(OcpiCdrDimension::into)
                .collect(),
        }
    }
}

impl From<Cdr> for v221::cdr::Cdr {
    fn from(cdr: Cdr) -> Self {
        Self {
            currency: cdr.currency,
            end_date_time: cdr.stop_date_time,
            start_date_time: cdr.start_date_time,
            last_updated: cdr.last_updated,
            cdr_location: cdr.location.into(),
            charging_periods: cdr
                .charging_periods
                .into_iter()
                .map(OcpiChargingPeriod::into)
                .collect(),
            tariffs: cdr.tariffs.into_iter().map(OcpiTariff::into).collect(),
            total_cost: Price {
                excl_vat: cdr.total_cost,
                incl_vat: None,
            },
            total_energy: cdr.total_energy,
            total_energy_cost: None,
            total_time: cdr.total_time,
            total_time_cost: None,
            total_fixed_cost: None,
            total_parking_time: cdr.total_parking_time,
            total_parking_cost: None,
            total_reservation_cost: None,
        }
    }
}

impl From<OcpiLocation> for v221::cdr::OcpiCdrLocation {
    fn from(value: OcpiLocation) -> Self {
        Self {
            country: value.country,
            time_zone: value.time_zone,
        }
    }
}

impl From<OcpiCdrDimension> for Option<v221::cdr::OcpiCdrDimension> {
    fn from(dimension: OcpiCdrDimension) -> Self {
        use v221::cdr::OcpiCdrDimension as OcpiCdrDimension221;

        let result = match dimension {
            OcpiCdrDimension::Time(time) => OcpiCdrDimension221::Time(time),
            OcpiCdrDimension::Energy(energy) => OcpiCdrDimension221::Energy(energy),
            OcpiCdrDimension::MaxCurrent(current) => OcpiCdrDimension221::MaxCurrent(current),
            OcpiCdrDimension::MinCurrent(current) => OcpiCdrDimension221::MinCurrent(current),
            OcpiCdrDimension::ParkingTime(parking) => OcpiCdrDimension221::ParkingTime(parking),
            // We can safely ignore the flat dimension since this can be determined from the tariff and
            // period time-stamps.
            OcpiCdrDimension::Flat(()) => return None,
        };

        Some(result)
    }
}
