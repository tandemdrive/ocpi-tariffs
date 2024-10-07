//! The Tariff object describes a tariff and its properties

use serde::{Deserialize, Serialize};
use v221::tariff::CompatibilityVat;

use crate::{
    null_default,
    ocpi::v221,
    types::{
        electricity::{Kw, Kwh},
        money::Money,
        time::{DateTime, DayOfWeek, OcpiDate, OcpiTime, SecondsRound},
    },
};

/// The Tariff object describes a tariff and its properties
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiTariff {
    /// The OCPI id of this tariff.
    pub id: String,

    /// Currency of this tariff, ISO 4217 Code
    pub currency: String,

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
    /// Minimum amount to be billed. This unit will be billed in this `step_size`
    /// blocks. For example: if type is time and `step_size` is 300, then time will
    /// be billed in blocks of 5 minutes, so if 6 minutes is used, 10 minutes (2
    /// blocks of `step_size`) will be billed
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
    /// Defined in kWh, `step_size` multiplier: 1 Wh
    Energy,
    /// Flat fee, no unit for `step_size`
    Flat,
    /// Time not charging: defined in hours, `step_size` multiplier: 1 second
    ParkingTime,
    /// Time charging: defined in hours, `step_size` multiplier: 1 second
    Time,
}

/// Indicates when a tariff applies
#[derive(Clone, Deserialize, Serialize)]
pub struct OcpiTariffRestriction {
    /// Start time of day, for example 13:30, valid from this time of the day.
    /// Must be in 24h format with leading zeros. Hour/Minute separator: “:” Regex
    pub start_time: Option<OcpiTime>,

    /// End time of day, for example 19:45, valid until this
    /// time of the day. Same syntax as `start_time`
    pub end_time: Option<OcpiTime>,

    /// Start date, for example: 2015-12-24, valid from this day
    pub start_date: Option<OcpiDate>,

    /// End date, for example: 2015-12-27, valid until thisday (excluding this day)
    pub end_date: Option<OcpiDate>,

    /// Minimum used energy in kWh, for example 20, valid from this amount of energy is used
    pub min_kwh: Option<Kwh>,

    /// Maximum used energy in kWh, for example 50, valid until this amount of energy is used
    pub max_kwh: Option<Kwh>,

    /// Minimum power in kW, for example 0, valid from this charging speed
    pub min_power: Option<Kw>,

    /// Maximum power in kW, for example 20, valid up to this charging speed
    pub max_power: Option<Kw>,

    /// Minimum duration in seconds, valid for a duration from x seconds
    pub min_duration: Option<SecondsRound>,

    /// Maximum duration in seconds, valid for a duration up to x seconds
    pub max_duration: Option<SecondsRound>,

    /// Which day(s) of the week this tariff is valid
    #[serde(deserialize_with = "null_default", default)]
    pub day_of_week: Vec<DayOfWeek>,
}

impl From<OcpiTariff> for v221::tariff::OcpiTariff {
    fn from(tariff: OcpiTariff) -> Self {
        Self {
            id: tariff.id,
            currency: tariff.currency,
            min_price: None,
            max_price: None,
            elements: tariff
                .elements
                .into_iter()
                .map(OcpiTariffElement::into)
                .collect(),
            start_date_time: tariff.start_date_time,
            end_date_time: tariff.end_date_time,
        }
    }
}

impl From<OcpiTariffRestriction> for v221::tariff::OcpiTariffRestriction {
    fn from(restriction: OcpiTariffRestriction) -> Self {
        Self {
            start_date: restriction.start_date,
            end_date: restriction.end_date,
            start_time: restriction.start_time,
            end_time: restriction.end_time,
            max_power: restriction.max_power,
            min_power: restriction.min_power,
            min_duration: restriction.min_duration,
            max_duration: restriction.max_duration,
            min_kwh: restriction.min_kwh,
            max_kwh: restriction.max_kwh,
            day_of_week: restriction.day_of_week,
            max_current: None,
            min_current: None,
            reservation: None,
        }
    }
}

impl From<TariffDimensionType> for v221::tariff::TariffDimensionType {
    fn from(ty: TariffDimensionType) -> Self {
        match ty {
            TariffDimensionType::Flat => Self::Flat,
            TariffDimensionType::Time => Self::Time,
            TariffDimensionType::Energy => Self::Energy,
            TariffDimensionType::ParkingTime => Self::ParkingTime,
        }
    }
}

impl From<OcpiTariffElement> for v221::tariff::OcpiTariffElement {
    fn from(element: OcpiTariffElement) -> Self {
        Self {
            restrictions: element.restrictions.map(OcpiTariffRestriction::into),
            price_components: element
                .price_components
                .into_iter()
                .map(OcpiPriceComponent::into)
                .collect(),
        }
    }
}

impl From<OcpiPriceComponent> for v221::tariff::OcpiPriceComponent {
    fn from(component: OcpiPriceComponent) -> Self {
        Self {
            component_type: TariffDimensionType::into(component.component_type),
            price: component.price,
            step_size: component.step_size,
            vat: CompatibilityVat::Unknown,
        }
    }
}
