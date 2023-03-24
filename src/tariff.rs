use crate::ocpi::tariff::{OcpiPriceComponent, OcpiTariff, OcpiTariffElement, TariffDimensionType};

use crate::restriction::{collect_restrictions, Restriction};
use crate::session::ChargePeriod;
use crate::types::{DateTime, Money};
use crate::{Error, Result};

use rust_decimal::Decimal;

#[derive(Debug)]
pub struct Tariffs(Vec<Tariff>);

impl Tariffs {
    pub fn new(tariffs: &[OcpiTariff]) -> Self {
        Self(
            tariffs
                .iter()
                .enumerate()
                .map(|(i, t)| Tariff::new(t, i))
                .collect(),
        )
    }

    pub fn active_tariff(&self, start_time: DateTime) -> Result<&Tariff> {
        self.0
            .iter()
            .find(|t| t.is_active(start_time))
            .ok_or(Error::NoValidTariff)
    }
}

#[derive(Debug)]
pub struct Tariff {
    pub tariff_index: usize,
    elements: Vec<TariffElement>,
    start_date_time: Option<DateTime>,
    end_date_time: Option<DateTime>,
}

impl Tariff {
    fn new(tariff: &OcpiTariff, tariff_index: usize) -> Self {
        let elements = tariff
            .elements
            .iter()
            .enumerate()
            .map(|(element_index, element)| TariffElement::new(element, element_index))
            .collect();

        Self {
            start_date_time: tariff.start_date_time,
            end_date_time: tariff.end_date_time,
            tariff_index,
            elements,
        }
    }

    pub fn active_components(&self, period: &ChargePeriod) -> PriceComponents {
        let mut components = PriceComponents::new();

        for tariff_element in self.elements.iter() {
            if !tariff_element.is_active(period) {
                continue;
            }

            if components.time.is_none() {
                components.time = tariff_element.components.time;
            }

            if components.parking.is_none() {
                components.parking = tariff_element.components.parking;
            }

            if components.energy.is_none() {
                components.energy = tariff_element.components.energy;
            }

            if components.flat.is_none() {
                components.flat = tariff_element.components.flat;
            }

            if components.has_all_components() {
                break;
            }
        }

        components
    }

    fn is_active(&self, start_time: DateTime) -> bool {
        if let Some(tariff_start_time) = self.start_date_time {
            return start_time >= tariff_start_time;
        }

        true
    }
}

#[derive(Debug)]
struct TariffElement {
    restrictions: Vec<Restriction>,
    components: PriceComponents,
}

impl TariffElement {
    fn new(ocpi_element: &OcpiTariffElement, element_index: usize) -> Self {
        let restrictions = if let Some(restrictions) = &ocpi_element.restrictions {
            collect_restrictions(restrictions)
        } else {
            Vec::new()
        };

        let mut components = PriceComponents::new();

        for ocpi_component in ocpi_element.price_components.iter() {
            let price_component = PriceComponent::new(ocpi_component, element_index);

            match ocpi_component.component_type {
                TariffDimensionType::Flat => components.flat.get_or_insert(price_component),
                TariffDimensionType::Time => components.time.get_or_insert(price_component),
                TariffDimensionType::ParkingTime => {
                    components.parking.get_or_insert(price_component)
                }
                TariffDimensionType::Energy => components.energy.get_or_insert(price_component),
            };
        }

        Self {
            restrictions,
            components,
        }
    }

    fn is_active(&self, period: &ChargePeriod) -> bool {
        for restriction in self.restrictions.iter() {
            if !restriction.instant_validity_exclusive(&period.start_instant) {
                return false;
            }

            if !restriction.period_validity(&period.period_data) {
                return false;
            }
        }

        true
    }

    fn is_active_at_end(&self, period: &ChargePeriod) -> bool {
        for restriction in self.restrictions.iter() {
            if !restriction.instant_validity_inclusive(&period.end_instant) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug)]
pub struct PriceComponents {
    pub flat: Option<PriceComponent>,
    pub energy: Option<PriceComponent>,
    pub parking: Option<PriceComponent>,
    pub time: Option<PriceComponent>,
}

impl PriceComponents {
    fn new() -> Self {
        Self {
            flat: None,
            energy: None,
            parking: None,
            time: None,
        }
    }

    pub fn has_all_components(&self) -> bool {
        self.flat.is_some()
            && self.energy.is_some()
            && self.parking.is_some()
            && self.time.is_some()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PriceComponent {
    pub tariff_element_index: usize,
    pub price: Money,
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
