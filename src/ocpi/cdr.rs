use crate::ocpi::tariff::Tariff;
use crate::ocpi::{DateTime, Number, Price};

/// The CDR object describes the Charging Session and its costs. How these costs are build up etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cdr {
    /// Start timestamp of the charging session
    pub start_date_time: DateTime,

    /// Stop timestamp of the charging session
    pub stop_date_time: DateTime,

    /// Currency of the CDR in ISO 4217 Code
    pub currency: String,

    /// List of relevant tariff elements
    pub tariffs: Vec<Tariff>,

    /// List of charging periods that make up this charging session> A session should consist of 1 or
    /// more periods, where each period has a different relevant Tariff
    pub charging_periods: Vec<OcpiChargingPeriod>,

    /// Total cost of this transaction
    pub total_cost: Number,

    pub total_fixed_cost: Option<Price>,

    /// Total energy charged, in kWh
    pub total_energy: Number,

    pub total_energy_cost: Option<Price>,

    /// Total time charging, in hours
    pub total_time: Number,

    pub total_time_cost: Option<Price>,

    /// Total time not charging, in hours
    pub total_parking_time: Option<Number>,

    pub total_parking_cost: Option<Price>,

    pub total_reservation_cost: Option<Price>,

    /// Timestamp when this CDR was last updated
    pub last_updated: DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcpiCdrDimension {
    /// Type of cdr dimension
    pub dimension_type: OcpiCdrDimensionType,
    /// Volume of the dimension consumed, measuredaccording to the dimension type
    pub volume: Number,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcpiChargingPeriod {
    /// Start timestamp of the charging period. Thisperiod ends when a next period starts, the
    /// lastperiod ends when the session ends
    pub start_date_time: DateTime,

    /// List of relevant values for this charging period
    pub dimensions: Vec<OcpiCdrDimension>,
}
