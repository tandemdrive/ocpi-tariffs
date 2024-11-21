use std::{collections::HashMap, fmt::Display, iter::once};

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike};
use rust_decimal::Decimal;

use crate::ocpi::v221::tariff::{OcpiTariff, OcpiTariffRestriction, TariffDimensionType};

#[derive(Debug)]
pub enum Warning {
    DimensionNotExhaustive {
        ty: TariffDimensionType,
        cases: Vec<()>,
    },
    ComponentIsRedundant {
        element_index: usize,
        component_index: usize,
    },
    ElementIsRedundant {
        element_index: usize,
    },
    UsesDateRestrictions {
        element_index: usize,
    },
}

impl Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UsesDateRestrictions { element_index } => write!(
                f,
                "Element at `$.elements[{element_index}]` uses `restrictions.start_date` or `restrictions.end_date`, \
                   consider using the top level `start_date` and `end_date`."
            ),
            Self::ComponentIsRedundant {
                element_index,
                component_index,
            } => {
                write!(f, "Component at `$.elements[{element_index}].price_components[{component_index}]` is redundant, consider removing it.")
            }
            Self::ElementIsRedundant { element_index } => {
                write!(
                    f,
                    "Element at `$.elements[{element_index}]` is redundant, consider removing it."
                )
            }
            Self::DimensionNotExhaustive { ty, .. } => {
                write!(
                    f,
                    "Dimension {ty:?} is not exhaustive, consider adding a fallback case."
                )
            }
        }
    }
}

/// Lint the provided tariff and produce a set of relevant warnings.
pub fn lint(tariff: &OcpiTariff) -> Vec<Warning> {
    let mut warnings = Vec::new();

    let mut energy_elements = Vec::new();
    let mut flat_elements = Vec::new();
    let mut time_elements = Vec::new();
    let mut parking_time_elements = Vec::new();

    // First we expand any element with multiple components into multiple
    // elements with a single components grouped by component type. This step
    // also marks any following components of the same type within the same
    // element redundant.

    for (element_index, element) in tariff.elements.iter().enumerate() {
        if element.price_components.is_empty() {
            warnings.push(Warning::ElementIsRedundant { element_index });
        }

        let mut has_energy = false;
        let mut has_flat = false;
        let mut has_time = false;
        let mut has_parking_time = false;

        for (component_index, component) in element.price_components.iter().enumerate() {
            match component.component_type {
                TariffDimensionType::Flat if !has_flat => {
                    flat_elements.push(UnaryElement {
                        component_index,
                        element_index,
                        restrictions: element.restrictions.clone(),
                    });

                    has_flat = true;
                }
                TariffDimensionType::Time if !has_time => {
                    time_elements.push(UnaryElement {
                        element_index,
                        component_index,
                        restrictions: element.restrictions.clone(),
                    });

                    has_time = true;
                }
                TariffDimensionType::Energy if !has_energy => {
                    energy_elements.push(UnaryElement {
                        element_index,
                        component_index,
                        restrictions: element.restrictions.clone(),
                    });

                    has_energy = true;
                }
                TariffDimensionType::ParkingTime if !has_parking_time => {
                    parking_time_elements.push(UnaryElement {
                        element_index,
                        component_index,
                        restrictions: element.restrictions.clone(),
                    });

                    has_parking_time = true;
                }
                _ => warnings.push(Warning::ComponentIsRedundant {
                    element_index,
                    component_index,
                }),
            }
        }
    }

    // Now for each component type we attempt to lint the restrictions.
    lint_restrictions(
        &mut energy_elements,
        TariffDimensionType::Energy,
        &mut warnings,
    );
    lint_restrictions(&mut flat_elements, TariffDimensionType::Flat, &mut warnings);
    lint_restrictions(
        &mut time_elements,
        TariffDimensionType::Energy,
        &mut warnings,
    );
    lint_restrictions(
        &mut parking_time_elements,
        TariffDimensionType::ParkingTime,
        &mut warnings,
    );

    let mut comp_counts = HashMap::new();

    for warn in &warnings {
        if let &Warning::ComponentIsRedundant { element_index, .. } = warn {
            let remaining = comp_counts
                .entry(element_index)
                .or_insert_with(|| tariff.elements[element_index].price_components.len());

            *remaining -= 1;
        }
    }

    for (el_idx, count) in comp_counts {
        // All components are redundant, mark the whole element as redundant.
        if count == 0 {
            warnings.retain(|w| match w {
                &Warning::ComponentIsRedundant { element_index, .. } if el_idx == element_index => {
                    false
                }
                _ => true,
            });

            warnings.push(Warning::ElementIsRedundant {
                element_index: el_idx,
            });
        }
    }

    warnings
}

struct UnaryElement {
    element_index: usize,
    component_index: usize,
    restrictions: Option<OcpiTariffRestriction>,
}

fn lint_restrictions(
    elements: &mut Vec<UnaryElement>,
    ty: TariffDimensionType,
    warnings: &mut Vec<Warning>,
) {
    // Define numeric bounds for each restriction range.
    let bounds = vec![
        // Energy
        Range::new(Some(Decimal::ZERO), None),
        // Time in seconds from midnight
        Range::new(Some(Decimal::ZERO), Some(Decimal::from(60 * 60 * 24))),
        // Date in number of days from CE.
        Range::new(
            Some(Decimal::ZERO),
            Some(Decimal::from(NaiveDate::MAX.num_days_from_ce())),
        ),
        // Duration in milliseconds
        Range::new(Some(Decimal::ZERO), None),
    ];

    let mut matrix = Matrix::new(bounds);

    for element in elements.iter() {
        let Some(restr) = &element.restrictions else {
            matrix.add_pattern(Pattern::new(
                vec![Range::wildcard(); 4],
                element.element_index,
                element.component_index,
            ));
            continue;
        };

        matrix.add_pattern(Pattern::new(
            vec![
                Range::new(restr.min_kwh.map(Into::into), restr.max_kwh.map(Into::into)),
                Range::new(
                    restr
                        .start_time
                        .map(|s| NaiveTime::from(s).num_seconds_from_midnight().into()),
                    restr
                        .end_time
                        .map(|s| NaiveTime::from(s).num_seconds_from_midnight().into()),
                ),
                Range::new(
                    restr
                        .start_date
                        .map(|s| NaiveDate::from(s).num_days_from_ce().into()),
                    restr
                        .end_date
                        .map(|s| NaiveDate::from(s).num_days_from_ce().into()),
                ),
                Range::new(
                    restr
                        .min_duration
                        .map(|m| Duration::from(m).num_milliseconds().into()),
                    restr
                        .max_duration
                        .map(|m| Duration::from(m).num_milliseconds().into()),
                ),
            ],
            element.element_index,
            element.component_index,
        ));
    }

    // Add a virtual wildcard element/pattern. If the wildcard is useful or
    // not redundant, it means that this dimension is not exhaustive.
    matrix.add_pattern(Pattern::wildcard(4));

    matrix.usefulness();

    for pattern in &matrix.patterns[..matrix.patterns.len() - 1] {
        let element_index = pattern
            .element_index
            .expect("pattern should have element index");

        let component_index = pattern
            .component_index
            .expect("pattern should have component index");

        if !pattern.is_usefull {
            warnings.push(Warning::ComponentIsRedundant {
                element_index,
                component_index,
            })
        }
    }

    let last = matrix
        .patterns
        .last()
        .expect("should have at least one wildcard");

    // If the trailing wildcard is useful it means all the elements above are non-exhaustive.
    if last.is_usefull {
        warnings.push(Warning::DimensionNotExhaustive { ty, cases: vec![] })
    }
}

#[derive(Debug)]
pub struct Matrix {
    bounds: Vec<Range>,
    patterns: Vec<Pattern>,
}

impl Matrix {
    fn new(bounds: Vec<Range>) -> Self {
        Self {
            bounds,
            patterns: Vec::new(),
        }
    }

    fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern)
    }

    /// Computes usefulness for the whole matrix and mark the patterns with usefulness.
    ///
    /// This algorithm is based on "Maranget, Luc. (2007). Warnings for pattern matching"
    fn usefulness(&mut self) {
        for i in 0..self.patterns.len() {
            let consider = (0..i).collect::<Vec<_>>();

            let mut witnesses = self.usefulness_rec(0, i, &consider);

            if !witnesses.is_empty() {
                self.patterns[i].is_usefull = true;

                witnesses.iter_mut().for_each(|v| v.reverse());

                self.patterns[i].witness = witnesses;
            }
        }
    }

    /// Computes recursive usefulness of a specific pattern with a specific component.
    ///
    /// For the relevant set of constructor which match `pattern` we should consider all the
    /// patterns in the list `consider`. With remaining list of patterns that matched, we recurse
    /// down and now evaluate constructors for the next column.
    ///
    /// Once we reach the last column we will decide if `pattern` is useful.
    fn usefulness_rec(&self, column: usize, pattern: usize, consider: &[usize]) -> Vec<Vec<Range>> {
        // If we arrived at the last column we should check if any patterns above are still
        // considered. If there are none, this pattern is useful.
        let Some(bounds) = self.bounds.get(column) else {
            return if consider.is_empty() {
                vec![Vec::new()]
            } else {
                Vec::new()
            };
        };

        let iter = once(&pattern)
            .chain(consider)
            .map(|&i| self.patterns[i].columns[column])
            .chain(once(Range::wildcard()));

        let mut witnesses = Vec::new();

        for constr in constructors(*bounds, iter) {
            let colpat = self.patterns[pattern].columns[column];

            if !colpat.contains(&constr) {
                continue;
            }

            let mut next_consider = Vec::new();

            for &i in consider {
                let colpat = self.patterns[i].columns[column];

                if !colpat.contains(&constr) {
                    continue;
                }

                next_consider.push(i)
            }

            for mut witness in self.usefulness_rec(column + 1, pattern, &next_consider) {
                witness.push(constr);

                witnesses.push(witness)
            }
        }

        witnesses
    }
}

/// Create a list of relevant constructors for a set of patterns.
///
/// An empty list of patterns should produce the constructor set:
/// `[..]`.
///
/// A list of patterns defined as `[3..4, 6..]` should produce the constructor set:
/// `[0..3, 3..4, 4..6, 6..]`
///
fn constructors(bounds: Range, ranges: impl Iterator<Item = Range>) -> Vec<Range> {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Point {
        NegInf,
        Value(Decimal),
        PosInf,
    }

    let mut points = ranges
        .into_iter()
        .flat_map(|r| {
            [
                r.lower.map(Point::Value).unwrap_or(Point::NegInf),
                r.higher.map(Point::Value).unwrap_or(Point::PosInf),
            ]
        })
        .collect::<Vec<_>>();

    points.sort();
    points.dedup();

    let mut ranges = Vec::new();
    let mut prev = None;

    for point in points {
        let lower = match prev {
            Some(Point::NegInf | Point::PosInf) => None,
            Some(Point::Value(value)) => Some(value),
            None => {
                prev = Some(point);
                continue;
            }
        };

        let higher = match point {
            Point::Value(value) => Some(value),
            _ => None,
        };

        match (lower, higher) {
            (None, Some(higher)) if Some(higher) == bounds.lower => {}
            (Some(lower), None) if Some(lower) == bounds.higher => {}
            _ => {
                ranges.push(Range::new(lower, higher));
            }
        }

        prev = Some(point);
    }

    ranges
}

#[derive(Debug)]
struct Pattern {
    columns: Vec<Range>,
    is_usefull: bool,
    witness: Vec<Vec<Range>>,
    element_index: Option<usize>,
    component_index: Option<usize>,
}

impl Pattern {
    fn wildcard(width: usize) -> Self {
        Self {
            columns: vec![Range::wildcard(); width],
            is_usefull: false,
            witness: Vec::new(),
            element_index: None,
            component_index: None,
        }
    }

    fn new(columns: Vec<Range>, element_index: usize, component_index: usize) -> Self {
        Self {
            columns,
            is_usefull: false,
            witness: Vec::new(),
            element_index: Some(element_index),
            component_index: Some(component_index),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Range {
    pub lower: Option<Decimal>,
    pub higher: Option<Decimal>,
}

impl Range {
    fn contains(&self, other: &Self) -> bool {
        match (self.lower, other.lower) {
            (None, _) => {}
            (Some(lhs), Some(rhs)) if lhs <= rhs => {}
            _ => return false,
        }

        match (self.higher, other.higher) {
            (None, _) => {}
            (Some(lhs), Some(rhs)) if lhs >= rhs => {}
            _ => return false,
        }

        true
    }
}

impl Default for Range {
    fn default() -> Self {
        Self::wildcard()
    }
}

impl Range {
    fn new(lower: Option<Decimal>, higher: Option<Decimal>) -> Self {
        Self { lower, higher }
    }

    fn wildcard() -> Self {
        Self {
            lower: None,
            higher: None,
        }
    }
}
