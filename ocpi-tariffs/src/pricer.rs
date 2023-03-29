use std::ops::Mul;

use crate::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff},
    session::ChargeSession,
    session::{ChargePeriod, PeriodData},
    tariff::{PriceComponent, PriceComponents, Tariffs},
    types::{
        electricity::Kwh,
        money::{Money, Price},
        number::Number,
    },
    Error, Result,
};

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;

pub struct Pricer {
    session: ChargeSession,
    tariffs: Tariffs,
}

impl Pricer {
    pub fn new(cdr: &Cdr, local_timezone: Tz) -> Self {
        Self {
            session: ChargeSession::new(cdr, local_timezone),
            tariffs: Tariffs::new(&cdr.tariffs),
        }
    }

    pub fn with_tariffs(cdr: &Cdr, tariffs: &[OcpiTariff], local_timezone: Tz) -> Self {
        Self {
            session: ChargeSession::new(cdr, local_timezone),
            tariffs: Tariffs::new(tariffs),
        }
    }

    pub fn build_report(&self) -> Result<Report> {
        let (tariff_index, tariff) = self
            .tariffs
            .active_tariff(self.session.start_date_time)
            .ok_or(Error::NoValidTariff)?;

        let mut periods = Vec::new();
        let mut step_size = StepSize::new();

        let mut total_energy = Kwh::zero();
        let mut total_charging_time = Duration::zero();
        let mut total_parking_time = Duration::zero();

        for (index, period) in self.session.periods.iter().enumerate() {
            let components = tariff.active_components(period);

            step_size.update(index, &components, period);

            let dimensions = Dimensions::new(components, &period.period_data);

            total_charging_time =
                total_charging_time + dimensions.time.volume.unwrap_or_else(Duration::zero);

            total_energy += dimensions.energy.volume.unwrap_or_else(Kwh::zero);

            total_parking_time = total_parking_time
                + dimensions
                    .parking_time
                    .volume
                    .unwrap_or_else(Duration::zero);

            periods.push(PeriodReport::new(period, dimensions));
        }

        let billed_charging_time = step_size.apply_time(&mut periods, total_charging_time);
        let billed_energy = step_size.apply_energy(&mut periods, total_energy);
        let billed_parking_time = step_size.apply_parking_time(&mut periods, total_parking_time);

        let mut total_energy_cost = Price::zero();
        let mut total_time_cost = Price::zero();
        let mut total_parking_cost = Price::zero();
        let mut total_fixed_cost = Price::zero();

        for period in &periods {
            let dimensions = &period.dimensions;

            total_energy_cost += dimensions.energy.cost();
            total_time_cost += dimensions.time.cost();
            total_parking_cost += dimensions.parking_time.cost();
            total_fixed_cost += dimensions.flat.cost();
        }

        let total_time = if let Some(first) = periods.first() {
            let last = periods.last().unwrap();
            last.end_date_time - first.start_date_time
        } else {
            Duration::zero()
        };

        let total_cost =
            total_time_cost + total_parking_cost + total_fixed_cost + total_energy_cost;

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
            total_reservation_cost: Price::zero(),
        };

        // eprintln!("{:#?}", report);

        Ok(report)
    }
}

#[derive(Debug)]
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

        if period.period_data.duration.is_some() {
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
        total: Duration,
        billed_volume: &mut Duration,
        step_size: u64,
    ) -> Duration {
        let total_seconds = Number::from(total.num_seconds());
        let step_size = Number::from(step_size);

        let priced_total_seconds = ((total_seconds / step_size).ceil() * step_size)
            .try_into()
            .expect("overflow");

        let priced_total = Duration::seconds(priced_total_seconds);
        let difference = priced_total - total;
        *billed_volume = *billed_volume + difference;

        priced_total
    }

    fn apply_time(&self, periods: &mut [PeriodReport], total: Duration) -> Duration {
        if let (Some((time_index, price)), None) = (self.time, self.parking_time) {
            let period = &mut periods[time_index];
            let volume = &mut period
                .dimensions
                .time
                .billed_volume
                .expect("dimension should have a volume");

            Self::duration_step_size(total, volume, price.step_size)
        } else {
            total
        }
    }

    fn apply_parking_time(&self, periods: &mut [PeriodReport], total: Duration) -> Duration {
        if let Some((parking_index, price)) = self.parking_time {
            let period = &mut periods[parking_index];
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

    fn apply_energy(&self, periods: &mut [PeriodReport], total: Kwh) -> Kwh {
        if let Some((energy_index, price)) = self.energy {
            let period = &mut periods[energy_index];
            let volume = &mut period
                .dimensions
                .energy
                .billed_volume
                .expect("dimension should have a volume");

            let step_size = Number::from(price.step_size);

            let billed = Kwh::from_watt_hours((total.watt_hours() / step_size).ceil() * step_size);
            *volume += total - billed;

            billed
        } else {
            total
        }
    }
}

/// Structure containing the charge session priced according to the specified tariff.
/// The fields prefixed `total` correspond to CDR fields with the same name.
#[derive(Debug)]
pub struct Report {
    /// Charge session details per period.
    pub periods: Vec<PeriodReport>,
    /// Index of the tariff that was found to be active.
    pub tariff_index: usize,
    /// Total sum of all the costs of this transaction in the specified currency.
    pub total_cost: Price,
    /// Total sum of all the cost related to duration of charging during this transaction, in the specified currency.
    pub total_time_cost: Price,
    /// Total duration of the charging session (including the duration of charging and not charging), in hours.
    pub total_time: Duration,
    /// Total duration of the charging session (excluding not charging), in hours.
    pub total_charging_time: Duration,
    pub billed_charging_time: Duration,
    /// Total sum of all the cost related to parking of this transaction, including fixed price components, in the specified currency.
    pub total_parking_cost: Price,
    /// Total duration of the charging session where the EV was not charging (no energy was transferred between EVSE and EV), in hours.
    pub total_parking_time: Duration,
    pub billed_parking_time: Duration,
    /// Total sum of all the cost of all the energy used, in the specified currency.
    pub total_energy_cost: Price,
    /// Total energy charged, in kWh.
    pub total_energy: Kwh,
    pub billed_energy: Kwh,
    /// Total sum of all the fixed costs in the specified currency, except fixed price components of parking and reservation. The cost not depending on amount of time/energy used etc. Can contain costs like a start tariff.
    pub total_fixed_cost: Price,
    /// Total sum of all the cost related to a reservation of a Charge Point, including fixed price components, in the specified currency.
    pub total_reservation_cost: Price,
}

#[derive(Debug)]
pub struct PeriodReport {
    pub start_date_time: DateTime<Utc>,
    pub end_date_time: DateTime<Utc>,
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
    pub fn cost(&self) -> Price {
        self.dimensions.time.cost()
            + self.dimensions.parking_time.cost()
            + self.dimensions.flat.cost()
            + self.dimensions.energy.cost()
    }
}

#[derive(Debug)]
pub struct Dimensions {
    pub flat: DimensionReport<()>,
    pub energy: DimensionReport<Kwh>,
    pub time: DimensionReport<Duration>,
    pub parking_time: DimensionReport<Duration>,
}

impl Dimensions {
    pub fn new(components: PriceComponents, data: &PeriodData) -> Self {
        Self {
            parking_time: DimensionReport::new(components.parking, data.parking_duration),
            time: DimensionReport::new(components.time, data.duration),
            energy: DimensionReport::new(components.energy, data.energy),
            flat: DimensionReport::new(components.flat, Some(())),
        }
    }
}

#[derive(Debug)]
pub struct DimensionReport<V> {
    pub price: Option<PriceComponent>,
    pub volume: Option<V>,
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

impl<V> DimensionReport<V>
where
    V: Mul<Money, Output = Money> + Copy,
{
    /// The total cost of this dimension during a period.
    pub fn cost(&self) -> Price {
        Price {
            incl_vat: self.cost_incl_vat(),
            excl_vat: self.cost_excl_vat(),
        }
    }
    /// The cost excluding VAT of this dimension during a period.
    pub fn cost_excl_vat(&self) -> Money {
        if let Some(volume) = self.billed_volume {
            let price = self.price.map_or_else(Money::zero, |c| c.price);
            volume * price
        } else {
            Money::zero()
        }
    }

    /// The cost including VAT of this dimension during a period.
    pub fn cost_incl_vat(&self) -> Money {
        if let Some(vat) = self.price.and_then(|c| c.vat) {
            self.cost_excl_vat() * vat
        } else {
            self.cost_excl_vat()
        }
    }
}
