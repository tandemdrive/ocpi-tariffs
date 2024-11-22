use crate::{
    lint::{lint, Warning},
    ocpi::v221::tariff::OcpiTariff,
};

pub fn normalize(tariff: &mut OcpiTariff) {
    let warns = lint(tariff);

    let mut remove_components = Vec::new();
    let mut remove_elements = Vec::new();

    for warn in warns {
        match warn {
            Warning::ElementIsRedundant { element_index } => remove_elements.push(element_index),
            Warning::ComponentIsRedundant {
                element_index,
                component_index,
            } => remove_components.push((element_index, component_index)),
            _ => {}
        }
    }

    remove_components.sort_unstable();
    remove_elements.sort_unstable();

    // Remove them in sorted reverse order for the indices to stay intact.
    for &(el, comp) in remove_components.iter().rev() {
        tariff.elements[el].price_components.remove(comp);
    }

    for &el in remove_elements.iter().rev() {
        tariff.elements.remove(el);
    }

    tariff.elements.retain(|v| !v.price_components.is_empty());
}
