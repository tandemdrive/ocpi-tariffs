use serde::Deserialize;

use crate::ocpi::tariff::OcpiTariff;

use crate::types::{
    electricity::{Ampere, Kw, Kwh},
    money::Price,
    time::{DateTime, HoursDecimal},
};

/// The CDR object describes the Charging Session and its costs. How these costs are build up etc.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Cdr {
    /// Start timestamp of the charging session
    pub start_date_time: DateTime,

    /// Stop timestamp of the charging session
    pub stop_date_time: DateTime,

    /// Currency of the CDR in ISO 4217 Code
    pub currency: String,

    /// List of relevant tariff elements
    pub tariffs: Vec<OcpiTariff>,

    /// List of charging periods that make up this charging session> A session should consist of 1 or
    /// more periods, where each period has a different relevant Tariff
    pub charging_periods: Vec<OcpiChargingPeriod>,

    /// Total cost of this transaction
    pub total_cost: Price,

    pub total_fixed_cost: Option<Price>,

    /// Total energy charged, in kWh
    pub total_energy: Kwh,

    pub total_energy_cost: Option<Price>,

    /// Total time charging, in hours
    pub total_time: HoursDecimal,

    pub total_time_cost: Option<Price>,

    /// Total time not charging, in hours
    pub total_parking_time: Option<HoursDecimal>,

    pub total_parking_cost: Option<Price>,

    pub total_reservation_cost: Option<Price>,

    /// Timestamp when this CDR was last updated
    pub last_updated: DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type", content = "volume")]
pub enum OcpiCdrDimension {
    Energy(Kwh),
    MaxCurrent(Ampere),
    MinCurrent(Ampere),
    MaxPower(Kw),
    MinPower(Kw),
    ParkingTime(HoursDecimal),
    ReservationTime(HoursDecimal),
    Time(HoursDecimal),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OcpiCdrDimensionType {
    Energy,
    MaxCurrent,
    MinCurrent,
    MaxPower,
    MinPower,
    ParkingTime,
    ReservationTime,
    Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OcpiChargingPeriod {
    /// Start timestamp of the charging period. Thisperiod ends when a next period starts, the
    /// lastperiod ends when the session ends
    pub start_date_time: DateTime,

    /// List of relevant values for this charging period
    pub dimensions: Vec<OcpiCdrDimension>,
}
