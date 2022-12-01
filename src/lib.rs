mod ocpi;
mod restriction;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, ParseError, Weekday};
use chrono_tz::Tz;
use ocpi::{
    cdr::{Cdr, OcpiChargingPeriod},
    tariff::{
        DayOfWeek, OcpiTariffElement, OcpiTariffRestriction, PriceComponent, TariffDimensionType,
    },
    DateTime, Number,
};

use restriction::{collect_restrictions, Restriction};

/// An event is a point in time where a price component for a certain dimension either becomes
/// active or inactive
pub struct Event<'t> {
    /// The time at which this event occured
    pub time: DateTime,
    /// Whether this event activates or deactivates the price component.
    pub kind: EventKind,
    /// The dimension for which this event applies.
    pub dimension: TariffDimensionType,
    /// The `TariffElement` for which this event applies.
    pub element: &'t OcpiTariffElement,
}

#[derive(Debug)]
struct TariffElement {
    restrictions: Vec<Restriction>,
    time_component: Option<PriceComponent>,
    parking_component: Option<PriceComponent>,
    flat_component: Option<PriceComponent>,
    energy_component: Option<PriceComponent>,
}

impl TariffElement {
    fn new(ocpi_element: &OcpiTariffElement) -> Result<Self, Error> {
        let restrictions = ocpi_element
            .restrictions
            .as_ref()
            .map(collect_restrictions)
            .transpose()?
            .unwrap_or_default();

        let mut element = Self {
            restrictions,
            time_component: None,
            parking_component: None,
            flat_component: None,
            energy_component: None,
        };

        for ocpi_component in ocpi_element.price_components.iter() {
            let component = match ocpi_component.component_type {
                TariffDimensionType::Time => &mut element.time_component,
                TariffDimensionType::ParkingTime => &mut element.parking_component,
                TariffDimensionType::Energy => &mut element.energy_component,
                TariffDimensionType::Flat => &mut element.flat_component,
            };

            component.get_or_insert_with(|| ocpi_component.clone());
        }

        Ok(element)
    }
}

pub struct ChargeState {
    local_timezone: Tz,
    date_time: DateTime,
    duration: Duration,
    total_energy: Option<Number>,
    max_current: Option<Number>,
    min_current: Option<Number>,
    max_power: Option<Number>,
    min_power: Option<Number>,
}

impl ChargeState {

    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Self {
        Self {
            local_timezone,
            date_time: cdr.start_date_time,
            duration: Duration::seconds(0),
            total_energy: None,
            max_current: None,
            min_current: None,
            max_power: None,
            min_power: None,
        }
    }

    fn local_time(&self) -> NaiveTime {
        self.date_time.with_timezone(&self.local_timezone).time()
    }

    fn local_date(&self) -> NaiveDate {
        self.date_time
            .with_timezone(&self.local_timezone)
            .date_naive()
    }

    fn local_weekday(&self) -> Weekday {
        self.date_time.with_timezone(&self.local_timezone).weekday()
    }
}

pub enum EventKind {
    Activated,
    Deactivated(Vec<Restriction>),
}

#[derive(Debug)]
pub enum Error {
    InvalidTimeZone(chrono_tz::ParseError),
    InvalidDateTime(chrono::ParseError),
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Self::InvalidDateTime(err)
    }
}

impl From<chrono_tz::ParseError> for Error {
    fn from(err: chrono_tz::ParseError) -> Self {
        Self::InvalidTimeZone(err)
    }
}
