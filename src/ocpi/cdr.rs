use crate::ocpi::tariff::Tariff;
use crate::ocpi::{DateTime, Number};

/// The CDR object describes the Charging Session and its costs. How these costs are build up etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cdr {
    /// Uniquely identifies the CDR within the CPOsplatform (and suboperator platforms
    pub id: String,

    /// Start timestamp of the charging session
    pub start_date_time: Option<DateTime>,

    /// Stop timestamp of the charging session
    pub stop_date_time: Option<DateTime>,

    /// Reference to a token, identified by the auth_id field of the Token
    pub auth_id: Option<String>,

    /// Method used for authentication
    pub auth_method: Option<AuthMethod>,

    /// Location where the charging session took place, including only the relevant EVSE and
    /// Connector
    pub location: Option<()>,

    /// Identification of the Meter inside the Charge Point
    pub meter_id: Option<String>,

    /// Currency of the CDR in ISO 4217 Code
    pub currency: Option<String>,

    /// List of relevant tariff elements
    pub tariffs: Vec<Tariff>,

    /// List of charging periods that make up this charging session> A session should consist of 1 or
    /// more periods, where each period has a different relevant Tariff
    pub charging_periods: Vec<ChargingPeriod>,

    /// Total cost of this transaction
    pub total_cost: Option<Number>,

    /// Total energy charged, in kWh
    pub total_energy: Option<Number>,

    /// Total time charging, in hours
    pub total_time: Option<Number>,

    /// Total time not charging, in hours
    pub total_parking_time: Option<Number>,

    /// Optional remark, can be used to provide addition human readable information to the CDR, for
    /// example: reason why a transaction was stopped
    pub remark: Option<String>,

    /// Timestamp when this CDR was last updated
    pub last_updated: DateTime,
}

/// Simplest representation of a CDR for storing it in the database
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdrDatabaseFields {
    /// Uniquely identifies the CDR within the CPOsplatform (and suboperator platforms
    pub id: String,

    /// Timestamp when this CDR was last updated
    pub last_updated: DateTime,
}

/// Token authentication method used for the charge session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    /// Authentication request from the eMSP
    AuthRequest,

    /// Whitelist used to authenticate, no request done to the eMSP
    Whitelist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdrDimension {
    /// Type of cdr dimension
    pub dimension_type: CdrDimensionType,
    /// Volume of the dimension consumed, measuredaccording to the dimension type
    pub volume: Number,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdrDimensionType {
    /// Defined in kWh, default step_size is 1 Wh
    Energy,

    /// Flat fee, no unit
    Flat,

    /// Defined in A (Ampere), Maximum current reached during charging session
    MaxCurrent,

    /// Defined in A (Ampere), Minimum current used during charging session
    MinCurrent,

    /// Time not charging: defined in hours, default step_size is 1 second
    ParkingTime,

    /// Time charging: defined in hours, default step_size is 1 second
    Time,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargingPeriod {
    /// Start timestamp of the charging period. Thisperiod ends when a next period starts, the
    /// lastperiod ends when the session ends
    start_date_time: DateTime,

    /// List of relevant values for this charging period
    dimensions: Vec<CdrDimension>,
}
