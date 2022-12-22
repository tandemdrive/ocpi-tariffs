use std::collections::HashMap;

use crate::ocpi::tariff::{OcpiPriceComponent, OcpiTariff, OcpiTariffElement, TariffDimensionType};

use crate::restriction::{collect_restrictions, Restriction};
use crate::session::ChargePeriod;
use crate::{Error, Result};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

pub struct Tariffs(Vec<Tariff>);

impl Tariffs {
    pub fn new(tariffs: &[OcpiTariff]) -> Result<Self> {
        let mut result = Vec::new();

        for (index, tariff) in tariffs.iter().enumerate() {
            result.push(Tariff::new(tariff, index)?);
        }

        Ok(Self(result))
    }

    pub fn active_tariff(&self, period: &ChargePeriod) -> Result<&Tariff> {
        self.0
            .iter()
            .find(|t| t.is_active(period))
            .ok_or(Error::NoValidTariff)
    }
}

pub struct Tariff {
    pub tariff_index: usize,
    elements: Vec<TariffElement>,
    start_date_time: Option<DateTime<Utc>>,
    end_date_time: Option<DateTime<Utc>>,
}

impl Tariff {
    fn new(tariff: &OcpiTariff, tariff_index: usize) -> Result<Self> {
        let elements = tariff
            .elements
            .iter()
            .enumerate()
            .map(|(element_index, element)| TariffElement::new(element, element_index))
            .collect::<Result<_>>()?;

        Ok(Self {
            start_date_time: tariff.start_date_time,
            end_date_time: tariff.end_date_time,
            tariff_index,
            elements,
        })
    }

    pub fn active_components(
        &self,
        period: &ChargePeriod,
    ) -> Result<HashMap<TariffDimensionType, PriceComponent>> {
        let mut components = HashMap::new();

        for tariff_element in self.elements.iter() {
            if !tariff_element.is_active(period)? {
                continue;
            }

            for (&dimension, &component) in tariff_element.components.iter() {
                components.entry(dimension).or_insert(component);
            }

            if components.len() == TariffDimensionType::NUM_VARIANTS {
                break;
            }
        }

        Ok(components)
    }

    fn is_active(&self, period: &ChargePeriod) -> bool {
        if let Some(start_date_time) = self.start_date_time {
            if period.start_instant.date_time < start_date_time {
                return false;
            }
        }

        if let Some(end_date_time) = self.end_date_time {
            if period.start_instant.date_time > end_date_time {
                return false;
            }
        }

        true
    }
}

#[derive(Debug)]
struct TariffElement {
    restrictions: Vec<Restriction>,
    components: HashMap<TariffDimensionType, PriceComponent>,
}

impl TariffElement {
    fn new(ocpi_element: &OcpiTariffElement, element_index: usize) -> Result<Self> {
        let restrictions = if let Some(restrictions) = &ocpi_element.restrictions {
            collect_restrictions(restrictions)?
        } else {
            Vec::new()
        };

        let mut components = HashMap::new();

        for ocpi_component in ocpi_element.price_components.iter() {
            components
                .entry(ocpi_component.component_type)
                .or_insert_with(|| PriceComponent::new(ocpi_component, element_index));
        }

        Ok(Self {
            restrictions,
            components,
        })
    }

    fn is_active(&self, period: &ChargePeriod) -> Result<bool> {
        for restriction in self.restrictions.iter() {
            if !restriction.instant_validity_exclusive(&period.start_instant)? {
                return Ok(false);
            }

            if !restriction.state_validity(&period.charge_state)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn is_active_at_end(&self, period: &ChargePeriod) -> Result<bool> {
        for restriction in self.restrictions.iter() {
            if !restriction.instant_validity_inclusive(&period.end_instant)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PriceComponent {
    pub tariff_element_index: usize,
    pub price: Decimal,
    pub vat: Option<Decimal>,
    pub step_size: u64,
}

impl PriceComponent {
    fn new(component: &OcpiPriceComponent, tariff_element_index: usize) -> Self {
        let &OcpiPriceComponent {
            price,
            vat,
            step_size,
            ..
        } = component;

        Self {
            tariff_element_index,
            price,
            vat,
            step_size,
        }
    }
}
