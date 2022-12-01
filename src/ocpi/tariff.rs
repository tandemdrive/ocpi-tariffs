//! The Tariff object describes a tariff and its properties

use crate::ocpi::{DateTime, DisplayText, Number, Price};

/// The Tariff object describes a tariff and its properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tariff {
    pub county_code: String,

    pub party_id: String,

    /// Uniquely identifies the tariff within the CPOs platform (and suboperator platforms).
    pub id: String,

    /// Currency of this tariff, ISO 4217 Code
    pub currency: String,

    pub tariff_type: Option<TariffType>,

    /// List of multi language alternative tariff info text
    pub tariff_alt_text: Vec<DisplayText>,

    /// Alternative URL to tariff info
    pub tariff_alt_url: Option<()>,

    pub min_price: Option<Price>,

    pub max_price: Option<Price>,

    /// List of tariff elements
    pub elements: Vec<OcpiTariffElement>,

    pub start_date_time: Option<DateTime>,

    pub end_date_time: Option<DateTime>,

    /// Details on the energy supplied with this tariff
    pub energy_mix: Option<()>,

    /// Timestamp when this Tariff was last updated (or created).
    pub last_updated: DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TariffType {
    AdHocPayment,
    ProfileCheap,
    ProfileFast,
    ProfileGreen,
    Regular,
}

/// Weekday enum
#[derive(Debug, Copy, PartialEq, Eq, Clone, Hash)]
pub enum DayOfWeek {
    /// Monday
    Monday,
    /// Tuesday
    Tuesday,
    /// Wednesday
    Wednesday,
    /// Thursday
    Thursday,
    /// Friday
    Friday,
    /// Saturday
    Saturday,
    /// Sunday
    Sunday,
}

/// Component of a tariff price
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceComponent {
    /// Type of tariff dimension
    pub component_type: TariffDimensionType,

    /// Price per unit (excluding VAT) for this tariff dimension
    pub price: Number,

    pub vat: Option<Number>,

    /// Minimum amount to be billed. This unit will be billed in this step_size
    /// blocks. For example: if type is time and step_size is 300, then time will
    /// be billed in blocks of 5 minutes, so if 6 minutes is used, 10 minutes (2
    /// blocks of step_size) will be billed
    pub step_size: u64,
}

/// Describes part of a tariff
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcpiTariffElement {
    /// List of price components that make up the pricing of this tariff
    pub price_components: Vec<PriceComponent>,

    /// Tariff restrictions object
    pub restrictions: Option<OcpiTariffRestriction>,
}

/// Type of tariff component
#[derive(Debug, Copy, PartialEq, Eq, Clone)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcpiTariffRestriction {
    /// Start time of day, for example 13:30, valid from this time of the day.
    /// Must be in 24h format with leading zeros. Hour/Minute separator: “:” Regex
    pub start_time: Option<String>,

    /// End time of day, for example 19:45, valid until this
    /// time of the day. Same syntax as start_time
    pub end_time: Option<String>,

    /// Start date, for example: 2015-12-24, valid from this day
    pub start_date: Option<String>,

    /// End date, for example: 2015-12-27, valid until thisday (excluding this day)
    pub end_date: Option<String>,

    /// Minimum used energy in kWh, for example 20, valid from this amount of energy is used
    pub min_kwh: Option<Number>,

    /// Maximum used energy in kWh, for example 50, valid until this amount of energy is used
    pub max_kwh: Option<Number>,

    pub min_current: Option<Number>,

    pub max_current: Option<Number>,

    /// Minimum power in kW, for example 0, valid from this charging speed
    pub min_power: Option<Number>,

    /// Maximum power in kW, for example 20, valid up to this charging speed
    pub max_power: Option<Number>,

    /// Minimum duration in seconds, valid for a duration from x seconds
    pub min_duration: Option<i64>,

    /// Maximum duration in seconds, valid for a duration up to x seconds
    pub max_duration: Option<i64>,

    /// Which day(s) of the week this tariff is valid
    pub day_of_week: Vec<DayOfWeek>,

    pub reservation: Option<ReservationRestrictionType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationRestrictionType {
    Reservation,
    ReservationExpiress,
}
