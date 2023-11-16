use crate::{
    ocpi::{
        cdr::Cdr,
        tariff::{CompatibilityVat, OcpiTariff},
    },
    session::{ChargePeriod, ChargeSession, PeriodData},
    tariff::{PriceComponent, PriceComponents, Tariffs},
    types::{
        electricity::Kwh,
        money::{Money, Price},
        number::Number,
        time::HoursDecimal,
    },
    Error, Result,
};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::Serialize;

/// Pricer that encapsulates a single charge-session and a list of tariffs.
/// To run the pricer call `build_report`. The resulting report contains the totals, subtotals and a breakdown of the
/// calculation.
///
/// Either specify a `Cdr` containing a list of tariffs.
/// ```ignore
/// let pricer = Pricer::new(cdr, Tz::Europe__Amsterdam);
/// let report = pricer.build_report();
/// ```
///
/// Or provide both the `Cdr` and a slice of `OcpiTariff`'s.
/// ```ignore
/// let pricer = Pricer::with_tariffs(cdr, tariffs, Tz::Europe__Amsterdam);
/// let report = pricer.build_report();
/// ```
pub struct Pricer {
    session: ChargeSession,
    tariffs: Tariffs,
}

impl Pricer {
    /// Instantiate the pricer with a `Cdr` that contains at least on tariff.
    /// Provide the `local_timezone` of the area where this charge session was priced.
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Self {
        Self {
            session: ChargeSession::new(cdr, local_timezone),
            tariffs: Tariffs::new(&cdr.tariffs),
        }
    }

    /// Instantiate the pricer with a `Cdr` and a slice that contains at least on tariff.
    /// Provide the `local_timezone` of the area where this charge session was priced.
    pub fn with_tariffs(cdr: &Cdr, tariffs: &[OcpiTariff], local_timezone: Tz) -> Self {
        Self {
            session: ChargeSession::new(cdr, local_timezone),
            tariffs: Tariffs::new(tariffs),
        }
    }

    /// Attempt to apply the first found valid tariff the charge session and build a report
    /// containing the results.
    pub fn build_report(&self) -> Result<Report> {
        let (tariff_index, tariff) = self
            .tariffs
            .active_tariff(self.session.start_date_time)
            .ok_or(Error::NoValidTariff)?;

        let mut periods = Vec::new();
        let mut step_size = StepSize::new();

        let mut total_energy = Kwh::zero();
        let mut total_charging_time = HoursDecimal::zero();
        let mut total_parking_time = HoursDecimal::zero();

        for (index, period) in self.session.periods.iter().enumerate() {
            let components = tariff.active_components(period);

            step_size.update(index, &components, period);

            let dimensions = Dimensions::new(components, &period.period_data);

            total_charging_time = total_charging_time
                .saturating_add(dimensions.time.volume.unwrap_or_else(HoursDecimal::zero));

            total_energy =
                total_energy.saturating_add(dimensions.energy.volume.unwrap_or_else(Kwh::zero));

            total_parking_time = total_parking_time.saturating_add(
                dimensions
                    .parking_time
                    .volume
                    .unwrap_or_else(HoursDecimal::zero),
            );

            periods.push(PeriodReport::new(period, dimensions));
        }

        let billed_charging_time = step_size.apply_time(&mut periods, total_charging_time);
        let billed_energy = step_size.apply_energy(&mut periods, total_energy);
        let billed_parking_time = step_size.apply_parking_time(&mut periods, total_parking_time);

        let mut total_energy_cost: Option<Price> = None;
        let mut total_time_cost: Option<Price> = None;
        let mut total_parking_cost: Option<Price> = None;
        let mut total_fixed_cost: Option<Price> = None;

        for period in &periods {
            let dimensions = &period.dimensions;

            total_energy_cost = match (total_energy_cost, dimensions.energy.cost()) {
                (None, None) => None,
                (total, period) => Some(
                    total
                        .unwrap_or_default()
                        .saturating_add(period.unwrap_or_default()),
                ),
            };

            total_time_cost = match (total_time_cost, dimensions.time.cost()) {
                (None, None) => None,
                (total, period) => Some(
                    total
                        .unwrap_or_default()
                        .saturating_add(period.unwrap_or_default()),
                ),
            };

            total_parking_cost = match (total_parking_cost, dimensions.parking_time.cost()) {
                (None, None) => None,
                (total, period) => Some(
                    total
                        .unwrap_or_default()
                        .saturating_add(period.unwrap_or_default()),
                ),
            };

            total_fixed_cost = match (total_fixed_cost, dimensions.flat.cost()) {
                (None, None) => None,
                (total, period) => Some(
                    total
                        .unwrap_or_default()
                        .saturating_add(period.unwrap_or_default()),
                ),
            };
        }

        let total_time = if let Some(first) = periods.first() {
            let last = periods.last().unwrap();
            (last
                .end_date_time
                .signed_duration_since(first.start_date_time))
            .into()
        } else {
            HoursDecimal::zero()
        };

        let total_cost = [
            total_time_cost,
            total_parking_cost,
            total_fixed_cost,
            total_energy_cost,
        ]
        .into_iter()
        .fold(None, |accum: Option<Price>, next| match (accum, next) {
            (None, None) => None,
            _ => Some(
                accum
                    .unwrap_or_default()
                    .saturating_add(next.unwrap_or_default()),
            ),
        });

        let report = Report {
            periods,
            tariff_index,
            total_cost,
            total_time_cost,
            total_charging_time,
            total_time,
            total_parking_cost,
            total_parking_time,
            total_energy_cost,
            total_energy,
            total_fixed_cost,
            billed_parking_time,
            billed_energy,
            billed_charging_time,
            total_reservation_cost: None,
        };

        Ok(report)
    }
}

struct StepSize {
    time: Option<(usize, PriceComponent)>,
    parking_time: Option<(usize, PriceComponent)>,
    energy: Option<(usize, PriceComponent)>,
}

impl StepSize {
    fn new() -> Self {
        Self {
            time: None,
            parking_time: None,
            energy: None,
        }
    }

    fn update(&mut self, index: usize, components: &PriceComponents, period: &ChargePeriod) {
        if period.period_data.energy.is_some() {
            if let Some(energy) = components.energy {
                self.energy = Some((index, energy));
            }
        }

        if period.period_data.charging_duration.is_some() {
            if let Some(time) = components.time {
                self.time = Some((index, time));
            }
        }

        if period.period_data.parking_duration.is_some() {
            if let Some(parking) = components.parking {
                self.parking_time = Some((index, parking));
            }
        }
    }

    fn duration_step_size(
        total_volume: HoursDecimal,
        period_billed_volume: &mut HoursDecimal,
        step_size: u64,
    ) -> HoursDecimal {
        if step_size > 0 {
            let total_seconds = total_volume.as_num_seconds_number();
            let step_size = Number::from(step_size);

            let total_billed_volume = HoursDecimal::from_seconds_number(
                total_seconds
                    .checked_div(step_size)
                    .ceil()
                    .saturating_mul(step_size),
            )
            .expect("overflow");

            let period_delta_volume = total_billed_volume.saturating_sub(total_volume);
            *period_billed_volume = period_billed_volume.saturating_add(period_delta_volume);

            total_billed_volume
        } else {
            total_volume
        }
    }

    fn apply_time(&self, periods: &mut [PeriodReport], total: HoursDecimal) -> HoursDecimal {
        if let (Some((time_index, price)), None) = (&self.time, &self.parking_time) {
            let period = &mut periods[*time_index];
            let volume = period
                .dimensions
                .time
                .billed_volume
                .as_mut()
                .expect("dimension should have a volume");

            Self::duration_step_size(total, volume, price.step_size)
        } else {
            total
        }
    }

    fn apply_parking_time(
        &self,
        periods: &mut [PeriodReport],
        total: HoursDecimal,
    ) -> HoursDecimal {
        if let Some((parking_index, price)) = &self.parking_time {
            let period = &mut periods[*parking_index];
            let volume = period
                .dimensions
                .parking_time
                .billed_volume
                .as_mut()
                .expect("dimension should have a volume");

            Self::duration_step_size(total, volume, price.step_size)
        } else {
            total
        }
    }

    fn apply_energy(&self, periods: &mut [PeriodReport], total_volume: Kwh) -> Kwh {
        if let Some((energy_index, price)) = &self.energy {
            if price.step_size > 0 {
                let period = &mut periods[*energy_index];
                let step_size = Number::from(price.step_size);

                let period_billed_volume = period
                    .dimensions
                    .energy
                    .billed_volume
                    .as_mut()
                    .expect("dimension should have a volume");

                let total_billed_volume = Kwh::from_watt_hours(
                    total_volume
                        .watt_hours()
                        .checked_div(step_size)
                        .ceil()
                        .saturating_mul(step_size),
                );

                let period_delta_volume = total_billed_volume.saturating_sub(total_volume);
                *period_billed_volume = period_billed_volume.saturating_add(period_delta_volume);

                return total_billed_volume;
            }
        }

        total_volume
    }
}

/// Structure containing the charge session priced according to the specified tariff.
/// The fields prefixed `total` correspond to CDR fields with the same name.
#[derive(Serialize)]
pub struct Report {
    /// Charge session details per period.
    pub periods: Vec<PeriodReport>,
    /// Index of the tariff that was found to be active.
    pub tariff_index: usize,
    /// Total sum of all the costs of this transaction in the specified currency.
    pub total_cost: Option<Price>,
    /// Total sum of all the cost related to duration of charging during this transaction, in the specified currency.
    pub total_time_cost: Option<Price>,
    /// Total duration of the charging session (including the duration of charging and not charging), in hours.
    pub total_time: HoursDecimal,
    /// Total duration of the charging session (excluding not charging), in hours.
    pub total_charging_time: HoursDecimal,
    /// The total charging time after applying step-size.
    pub billed_charging_time: HoursDecimal,
    /// Total sum of all the cost related to parking of this transaction, including fixed price components, in the specified currency.
    pub total_parking_cost: Option<Price>,
    /// Total duration of the charging session where the EV was not charging (no energy was transferred between EVSE and EV), in hours.
    pub total_parking_time: HoursDecimal,
    /// The total parking time after applying step-size
    pub billed_parking_time: HoursDecimal,
    /// Total sum of all the cost of all the energy used, in the specified currency.
    pub total_energy_cost: Option<Price>,
    /// Total energy charged, in kWh.
    pub total_energy: Kwh,
    /// The total energy after applying step-size.
    pub billed_energy: Kwh,
    /// Total sum of all the fixed costs in the specified currency, except fixed price components of parking and reservation. The cost not depending on amount of time/energy used etc. Can contain costs like a start tariff.
    pub total_fixed_cost: Option<Price>,
    /// Total sum of all the cost related to a reservation of a Charge Point, including fixed price components, in the specified currency.
    pub total_reservation_cost: Option<Price>,
}

/// A report for a single period that occurred during a session.
#[derive(Serialize)]
pub struct PeriodReport {
    /// The start time of this period.
    pub start_date_time: DateTime<Utc>,
    /// The end time of this period.
    pub end_date_time: DateTime<Utc>,
    /// A structure that contains results per dimension.
    pub dimensions: Dimensions,
}

impl PeriodReport {
    fn new(period: &ChargePeriod, dimensions: Dimensions) -> Self {
        Self {
            start_date_time: period.start_instant.date_time,
            end_date_time: period.end_instant.date_time,
            dimensions,
        }
    }

    /// The total cost of all dimensions in this period.
    pub fn cost(&self) -> Option<Price> {
        [
            self.dimensions.time.cost(),
            self.dimensions.parking_time.cost(),
            self.dimensions.flat.cost(),
            self.dimensions.energy.cost(),
        ]
        .into_iter()
        .fold(None, |accum, next| {
            if accum.is_none() && next.is_none() {
                None
            } else {
                Some(
                    accum
                        .unwrap_or_default()
                        .saturating_add(next.unwrap_or_default()),
                )
            }
        })
    }
}

/// A structure containing a report for each dimension.
#[derive(Serialize)]
pub struct Dimensions {
    /// The flat dimension.
    pub flat: DimensionReport<()>,
    /// The energy dimension.
    pub energy: DimensionReport<Kwh>,
    /// The time dimension.
    pub time: DimensionReport<HoursDecimal>,
    /// The parking time dimension.
    pub parking_time: DimensionReport<HoursDecimal>,
}

impl Dimensions {
    pub(crate) fn new(components: PriceComponents, data: &PeriodData) -> Self {
        Self {
            parking_time: DimensionReport::new(
                components.parking,
                data.parking_duration.map(Into::into),
            ),
            time: DimensionReport::new(components.time, data.charging_duration.map(Into::into)),
            energy: DimensionReport::new(components.energy, data.energy),
            flat: DimensionReport::new(components.flat, Some(())),
        }
    }
}

#[derive(Serialize)]
/// A report for a single dimension during a single period.
pub struct DimensionReport<V> {
    /// The price component that was active during this period for this dimension.
    /// It could be that no price component was active during this period for this dimension in
    /// which case `price` is `None`.
    pub price: Option<PriceComponent>,
    /// The volume of this dimension during this period, as received in the provided charge detail record.
    /// It could be that no volume was provided during this period for this dimension in which case
    /// the `volume` is `None`.
    pub volume: Option<V>,
    /// This field contains the optional value of `volume` after a potential step size was applied.
    /// Step size is applied over the total volume during the whole session of a dimension. But the
    /// resulting additional volume should be billed according to the price component in this
    /// period.
    ///
    /// If no step-size was applied for this period, the volume is exactly equal to the `volume`
    /// field.
    pub billed_volume: Option<V>,
}

impl<V> DimensionReport<V>
where
    V: Copy,
{
    fn new(price_component: Option<PriceComponent>, volume: Option<V>) -> Self {
        Self {
            price: price_component,
            volume,
            billed_volume: volume,
        }
    }
}

impl<V: Dimension> DimensionReport<V> {
    /// The total cost of this dimension during a period.
    pub fn cost(&self) -> Option<Price> {
        if let (Some(volume), Some(price)) = (self.billed_volume, self.price) {
            let excl_vat = volume.cost(price.price);

            let incl_vat = match price.vat {
                CompatibilityVat::Vat(Some(vat)) => Some(excl_vat.apply_vat(vat)),
                CompatibilityVat::Vat(None) => Some(excl_vat),
                CompatibilityVat::Unknown => None,
            };

            Some(Price { excl_vat, incl_vat })
        } else {
            None
        }
    }
}

/// An OCPI tariff dimension
pub trait Dimension: Copy {
    /// The cost of this dimension at a certain price.
    fn cost(&self, price: Money) -> Money;
}

impl Dimension for Kwh {
    fn cost(&self, price: Money) -> Money {
        price.kwh_cost(*self)
    }
}

impl Dimension for () {
    fn cost(&self, price: Money) -> Money {
        price
    }
}

impl Dimension for HoursDecimal {
    fn cost(&self, price: Money) -> Money {
        price.time_cost(*self)
    }
}
