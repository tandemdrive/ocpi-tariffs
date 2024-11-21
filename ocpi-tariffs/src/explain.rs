use crate::{
    ocpi::v221::tariff::{OcpiTariff, OcpiTariffRestriction, TariffDimensionType},
    types::money::Money,
};

#[derive(Debug)]
pub struct Explain {
    pub elements: Vec<ExplainElement>,
}

#[derive(Debug)]
pub struct ExplainElement {
    pub restrictions: Vec<String>,
    pub components: ExplainComponents,
}

#[derive(Debug, Default)]
pub struct ExplainComponents {
    pub energy: Option<Money>,
    pub flat: Option<Money>,
    pub time: Option<Money>,
    pub parking_time: Option<Money>,
}

pub fn explain(tariff: &OcpiTariff) -> Explain {
    let mut elements = Vec::new();

    for element in &tariff.elements {
        let mut components = ExplainComponents::default();

        for component in &element.price_components {
            match component.component_type {
                TariffDimensionType::Flat => components.flat = Some(component.price.with_scale(2)),
                TariffDimensionType::Time => components.time = Some(component.price.with_scale(2)),
                TariffDimensionType::Energy => {
                    components.energy = Some(component.price.with_scale(2))
                }
                TariffDimensionType::ParkingTime => {
                    components.parking_time = Some(component.price.with_scale(2))
                }
            }
        }

        let restrictions = element
            .restrictions
            .as_ref()
            .map(|restr| explain_restrictions(restr))
            .unwrap_or_default();

        elements.push(ExplainElement {
            restrictions,
            components,
        });
    }

    Explain { elements }
}

/// Explain the given restriction.
pub fn explain_restrictions(restr: &OcpiTariffRestriction) -> Vec<String> {
    let mut explains = Vec::new();

    if let Some((min_kwh, max_kwh)) = restr.min_kwh.zip(restr.max_kwh) {
        explains.push(format!(
            "total energy is between {} and {} kWh",
            min_kwh.normalize(),
            max_kwh.normalize()
        ));
    } else if let Some(min_kwh) = restr.min_kwh {
        explains.push(format!("total energy exceeds {} kWh", min_kwh.normalize()));
    } else if let Some(max_kwh) = restr.max_kwh {
        explains.push(format!(
            "total energy is less than {} kWh",
            max_kwh.normalize()
        ));
    }

    if let Some((start_time, end_time)) = restr.start_time.zip(restr.end_time) {
        explains.push(format!("between {} and {}", start_time, end_time));
    } else if let Some(start_time) = restr.start_time {
        explains.push(format!("after {}", start_time));
    } else if let Some(end_time) = restr.end_time {
        explains.push(format!("before {}", end_time));
    }

    if let Some((min_duration, max_duration)) = restr.min_duration.zip(restr.max_duration) {
        explains.push(format!(
            "session duration is between {} and {} hours",
            min_duration.hours(),
            max_duration.hours()
        ));
    } else if let Some(min_duration) = restr.min_duration {
        explains.push(format!(
            "session duration exceeds {} hours",
            min_duration.hours(),
        ));
    } else if let Some(max_duration) = restr.max_duration {
        explains.push(format!(
            "session duration is less than {} hours",
            max_duration.hours(),
        ));
    }

    if let Some((start_date, end_date)) = restr.start_date.zip(restr.end_date) {
        explains.push(format!("between {} and {}", start_date, end_date));
    } else if let Some(start_date) = restr.start_date {
        explains.push(format!("after {}", start_date));
    } else if let Some(end_date) = restr.end_date {
        explains.push(format!("before {}", end_date));
    }

    explains
}
