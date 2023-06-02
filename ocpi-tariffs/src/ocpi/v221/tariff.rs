//! The Tariff object describes a tariff and its properties

use serde::{Deserialize, Serialize};

use crate::types::{
    electricity::{Ampere, Kw, Kwh},
    money::{Money, Price, Vat},
    time::{DateTime, DayOfWeek, OcpiDate, OcpiTime, SecondsRound},
};

/// The Tariff object describes a tariff and its properties
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiTariff {
    /// Code designating in which country this country is active.
    pub country_code: String,

    /// Currency of this tariff, ISO 4217 Code
    pub currency: String,

    /// The minimum amount that this tariff will cost.
    pub min_price: Option<Price>,

    /// The maximum amount that this tariff will cost.
    pub max_price: Option<Price>,

    /// List of tariff elements
    pub elements: Vec<OcpiTariffElement>,

    /// Start time when this tariff becomes active.
    pub start_date_time: Option<DateTime>,

    /// End time when this tariff becomes active.
    pub end_date_time: Option<DateTime>,
}

/// Component of a tariff price.
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiPriceComponent {
    /// Type of tariff dimension
    #[serde(rename = "type")]
    pub component_type: TariffDimensionType,

    /// Price per unit (excluding VAT) for this tariff dimension
    pub price: Money,

    /// Optionally specify a VAT percentage for this component.
    pub vat: Option<Vat>,

    /// Minimum amount to be billed. This unit will be billed in this step_size
    /// blocks. For example: if type is time and step_size is 300, then time will
    /// be billed in blocks of 5 minutes, so if 6 minutes is used, 10 minutes (2
    /// blocks of step_size) will be billed
    pub step_size: u64,
}

/// Describes part of a tariff
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiTariffElement {
    /// List of price components that make up the pricing of this tariff
    pub price_components: Vec<OcpiPriceComponent>,

    /// Tariff restrictions object
    pub restrictions: Option<OcpiTariffRestriction>,
}

/// Type of tariff component
#[derive(Debug, Copy, PartialEq, Eq, Clone, Hash, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TariffDimensionType {
    /// Defined in kWh, step_size multiplier: 1 Wh
    Energy,
    /// Flat fee, no unit for step_size
    Flat,
    /// Time not charging: defined in hours, step_size multiplier: 1 second
    ParkingTime,
    /// Time charging: defined in hours, step_size multiplier: 1 second
    Time,
}

/// Indicates when a tariff applies
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiTariffRestriction {
    /// Start time of day, for example 13:30, valid from this time of the day.
    /// Must be in 24h format with leading zeros. Hour/Minute separator: “:” Regex
    pub start_time: Option<OcpiTime>,

    /// End time of day, for example 19:45, valid until this
    /// time of the day. Same syntax as start_time
    pub end_time: Option<OcpiTime>,

    /// Start date, for example: 2015-12-24, valid from this day
    pub start_date: Option<OcpiDate>,

    /// End date, for example: 2015-12-27, valid until thisday (excluding this day)
    pub end_date: Option<OcpiDate>,

    /// Minimum used energy in kWh, for example 20, valid from this amount of energy is used
    pub min_kwh: Option<Kwh>,

    /// Maximum used energy in kWh, for example 50, valid until this amount of energy is used
    pub max_kwh: Option<Kwh>,

    /// The minimum current in A.
    pub min_current: Option<Ampere>,

    /// The maximum current in A.
    pub max_current: Option<Ampere>,

    /// Minimum power in kW, for example 0, valid from this charging speed
    pub min_power: Option<Kw>,

    /// Maximum power in kW, for example 20, valid up to this charging speed
    pub max_power: Option<Kw>,

    /// Minimum duration in seconds, valid for a duration from x seconds
    pub min_duration: Option<SecondsRound>,

    /// Maximum duration in seconds, valid for a duration up to x seconds
    pub max_duration: Option<SecondsRound>,

    /// Which day(s) of the week this tariff is valid
    #[serde(default)]
    pub day_of_week: Vec<DayOfWeek>,

    /// Whether this tariff applies for reservation.
    pub reservation: Option<ReservationRestrictionType>,
}

/// The type of reservation a tariff applies to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReservationRestrictionType {
    /// The tariff applies when the charge session is reserved.
    Reservation,
    /// The tariff applies when the reservation expires.
    ReservationExpires,
}
