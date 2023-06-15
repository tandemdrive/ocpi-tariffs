use serde::{Deserialize, Serialize};

use super::tariff::OcpiTariff;

use crate::null_default;

use crate::types::{
    electricity::{Ampere, Kw, Kwh},
    money::Price,
    time::{DateTime, HoursDecimal},
};

/// The CDR object describes the Charging Session and its costs. How these costs are build up etc.
#[derive(Clone, Deserialize, Serialize)]
pub struct Cdr {
    /// Start timestamp of the charging session.
    pub start_date_time: DateTime,

    /// Stop timestamp of the charging session.
    pub end_date_time: DateTime,

    /// Currency of the CDR in ISO 4217 Code.
    pub currency: String,

    /// List of relevant tariff elements.
    #[serde(deserialize_with = "null_default", default)]
    pub tariffs: Vec<OcpiTariff>,

    /// List of charging periods that make up this charging session> A session should consist of 1 or
    /// more periods, where each period has a different relevant Tariff.
    pub charging_periods: Vec<OcpiChargingPeriod>,

    /// Total cost of this transaction.
    pub total_cost: Price,

    /// Total cost of the flat dimension.
    pub total_fixed_cost: Option<Price>,

    /// Total energy charged, in kWh.
    pub total_energy: Kwh,

    /// Total cost related to the energy dimension.
    pub total_energy_cost: Option<Price>,

    /// Total time charging, in hours
    pub total_time: HoursDecimal,

    /// Total cost related to the charging time dimension.
    pub total_time_cost: Option<Price>,

    /// Total time not charging, in hours
    pub total_parking_time: Option<HoursDecimal>,

    /// Total cost related to the parking time dimension.
    pub total_parking_cost: Option<Price>,

    /// Total cost related to reservation time.
    pub total_reservation_cost: Option<Price>,

    /// Timestamp when this CDR was last updated
    pub last_updated: DateTime,
}

/// The volume that has been consumed for a specific dimension during a charging period.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type", content = "volume")]
pub enum OcpiCdrDimension {
    /// Consumed energy in `kWh`.
    Energy(Kwh),
    /// The peak current, in 'A', during this period.
    MaxCurrent(Ampere),
    /// The lowest current, in `A`, during this period.
    MinCurrent(Ampere),
    /// The maximum power, in 'kW', reached during this period.
    MaxPower(Kw),
    /// The minimum power, in 'kW', reached during this period.
    MinPower(Kw),
    /// The parking time, in hours, consumed in this period.
    ParkingTime(HoursDecimal),
    /// The reservation time, in hours, consumed in this period.
    ReservationTime(HoursDecimal),
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
