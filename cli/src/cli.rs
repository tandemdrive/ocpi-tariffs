use std::{borrow::Cow, fmt::Display, fs::File, io::stdin, ops::Mul, path::PathBuf, process::exit};

use chrono::DateTime;
use chrono_tz::Tz;
use clap::{Args, Parser, Subcommand};
use console::style;
use ocpi_tariffs::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff},
    pricer::{DimensionReport, Pricer, Report},
    types::{
        electricity::Kwh,
        money::{Money, Price, Vat},
        time::HoursDecimal,
    },
};
use tabled::{
    format::Format,
    object::{Rows, Segment},
    width::MinWidth,
    Alignment, Modify, Panel, Style, Table, Tabled,
};

use crate::{error::Error, Result};

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) {
        if let Err(err) = self.command.run() {
            eprintln!("{}", err);
            exit(1);
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Validate a given charge detail record (CDR) against either a provided tariff structure or
    /// a tariff that is contained in the CDR itself.
    ///
    /// This command will show the differences between the calculated totals and the totals
    /// contained in the provided CDR.
    Validate(Validate),
    /// Analyze a given charge detail record (CDR) against either a provided tariff structure or a
    /// tariff that is contained in the CDR itself.
    ///
    /// This command will show you a breakdown of all the calculated costs.
    Analyze(Analyze),
}

impl Command {
    fn run(self) -> Result<()> {
        match self {
            Self::Validate(args) => args.run(),
            Self::Analyze(args) => args.run(),
        }
    }
}

#[derive(Args)]
pub struct TariffArgs {
    /// A path to the charge detail record in json format.
    ///
    /// If no path is provided the CDR is read from standard in.
    #[arg(short = 'c', long)]
    cdr: Option<PathBuf>,
    /// A path to the tariff structure in json format.
    ///
    /// If no path is provided, then the tariff is inferred to be contained inside the
    /// provided CDR. If the CDR contains multiple tariff structures, the first valid tariff
    /// will be used.
    #[arg(short = 't', long)]
    tariff: Option<PathBuf>,
    /// Timezone for evaluating any local times contained in the tariff structure.
    #[arg(short = 'z', long, default_value = "Europe/Amsterdam")]
    timezone: Tz,
}

impl TariffArgs {
    fn cdr_name(&self) -> Cow<'_, str> {
        self.cdr.as_ref().map_or("<stdin>".into(), |c| {
            c.file_name().unwrap().to_string_lossy()
        })
    }

    fn tariff_name(&self) -> Cow<'_, str> {
        self.tariff.as_ref().map_or("<CDR-tariff>".into(), |c| {
            c.file_name().unwrap().to_string_lossy()
        })
    }

    fn load_all(&self) -> Result<(Report, Cdr, Option<OcpiTariff>)> {
        let cdr: Cdr = if let Some(cdr_path) = &self.cdr {
            let file = File::open(cdr_path).map_err(|e| Error::file(cdr_path.clone(), e))?;
            serde_json::from_reader(&file)
                .map_err(|e| Error::deserialize(cdr_path.display(), "CDR", e))?
        } else {
            let mut stdin = stdin().lock();
            serde_json::from_reader(&mut stdin)
                .map_err(|e| Error::deserialize("<stdin>", "CDR", e))?
        };

        let tariff: Option<OcpiTariff> = if let Some(path) = &self.tariff {
            let file = File::open(path).map_err(|e| Error::file(path.clone(), e))?;
            serde_json::from_reader(&file)
                .map_err(|e| Error::deserialize(path.display(), "tariff", e))?
        } else {
            None
        };

        let pricer = if let Some(tariff) = tariff.clone() {
            Pricer::with_tariffs(&cdr, &[tariff], self.timezone)
        } else {
            Pricer::new(&cdr, self.timezone)
        };

        let report = pricer.build_report().map_err(Error::Internal)?;

        Ok((report, cdr, tariff))
    }
}

#[derive(Parser)]
pub struct Validate {
    #[command(flatten)]
    args: TariffArgs,
}

#[derive(Tabled)]
struct ValidateRow {
    #[tabled(rename = "Property")]
    property: String,
    #[tabled(rename = "Calculated")]
    calculated: String,
    #[tabled(rename = "CDR")]
    cdr: String,
    #[tabled(rename = "Valid")]
    valid: bool,
}

struct ValidateTable {
    rows: Vec<ValidateRow>,
}

impl ValidateTable {
    pub fn row<T: Display + Eq + Default + Clone>(
        &mut self,
        report: T,
        cdr: Option<T>,
        name: &str,
    ) {
        let valid = report == cdr.clone().unwrap_or_default();

        let row = ValidateRow {
            property: name.to_string(),
            calculated: report.to_string(),
            cdr: cdr.map_or("<missing>".into(), |s| s.to_string()),
            valid,
        };

        self.rows.push(row);
    }

    pub fn price_row(&mut self, report: Price, cdr: Option<Price>, name: &str) {
        self.row(
            report.excl_vat,
            cdr.map(|s| s.excl_vat),
            &format!("{} excl. VAT", name),
        );
        self.row(
            report.incl_vat,
            cdr.map(|s| s.incl_vat),
            &format!("{} incl. VAT", name),
        );
    }

    pub fn valid_rows(&self) -> Vec<bool> {
        self.rows.iter().map(|r| r.valid).collect()
    }
}

impl Validate {
    fn run(self) -> Result<()> {
        let (report, cdr, _) = self.args.load_all()?;

        println!(
            "\n{} `{}` with tariff `{}`, using timezone `{}`:",
            style("Validating").green().bold(),
            style(self.args.cdr_name()).blue(),
            style(self.args.tariff_name()).blue(),
            style(self.args.timezone).blue(),
        );

        let mut table = ValidateTable { rows: Vec::new() };

        table.row(report.total_time.into(), Some(cdr.total_time), "Total Time");
        table.row(
            report.total_parking_time.into(),
            cdr.total_parking_time,
            "Total Parking time",
        );
        table.row(
            report.total_energy.with_scale(),
            Some(cdr.total_energy),
            "Total Energy",
        );

        table.price_row(
            report.total_cost.with_scale(),
            Some(cdr.total_cost),
            "Total Cost",
        );

        table.price_row(
            report.total_time_cost.with_scale(),
            cdr.total_time_cost,
            "Total Time cost",
        );
        table.price_row(
            report.total_fixed_cost.with_scale(),
            cdr.total_fixed_cost,
            "Total Fixed cost",
        );
        table.price_row(
            report.total_energy_cost.with_scale(),
            cdr.total_energy_cost,
            "Total Energy cost",
        );
        table.price_row(
            report.total_parking_cost.with_scale(),
            cdr.total_parking_cost,
            "Total Parking cost",
        );

        let valid = table.valid_rows();
        let all_valid = valid.iter().all(|&s| s);

        let format_valid = Modify::new(Rows::new(..)).with(Format::with_index(|row, (i, _)| {
            let row = style(row);
            if i == 0 {
                row.bold().to_string()
            } else if valid[i - 1] {
                row.green().to_string()
            } else {
                row.red().to_string()
            }
        }));

        println!(
            "{}",
            Table::new(table.rows)
                .with(Style::modern())
                .with(format_valid)
        );

        if all_valid {
            println!(
                "Calculation {} all totals in the CDR.\n",
                style("matches").green().bold()
            );
        } else {
            println!(
                "Calculation {} all totals in the CDR.\n",
                style("does not match").red().bold()
            );

            exit(1);
        }

        Ok(())
    }
}

#[derive(Parser)]
pub struct Analyze {
    #[command(flatten)]
    args: TariffArgs,
}

impl Analyze {
    fn run(self) -> Result<()> {
        let (report, _, _) = self.args.load_all()?;

        println!(
            "\n{} `{}` with tariff `{}`, using timezone `{}`:",
            style("Analyzing").green().bold(),
            style(self.args.cdr_name()).blue(),
            style(self.args.tariff_name()).blue(),
            style(self.args.timezone).blue(),
        );

        let mut energy: PeriodTable<Kwh> = PeriodTable::new("Energy");
        let mut parking: PeriodTable<HoursDecimal> = PeriodTable::new("Parking time");
        let mut time: PeriodTable<HoursDecimal> = PeriodTable::new("Charging Time");
        let mut flat: PeriodTable<UnitDisplay> = PeriodTable::new("Flat");

        for period in report.periods.iter() {
            let start_time = period.start_date_time.with_timezone(&self.args.timezone);

            energy.row(&period.dimensions.energy, start_time);
            parking.row(&period.dimensions.parking_time, start_time);
            time.row(&period.dimensions.time, start_time);
            flat.row(&period.dimensions.flat, start_time);
        }

        println!("{}", energy.into_table());
        println!("{}", parking.into_table());
        println!("{}", time.into_table());
        println!("{}", flat.into_table());

        Ok(())
    }
}

pub struct PeriodTable<V: Display> {
    name: String,
    rows: Vec<PeriodComponent<V>>,
}

impl<V: Display> PeriodTable<V> {
    pub fn new(name: &str) -> Self {
        Self {
            rows: Vec::new(),
            name: name.to_string(),
        }
    }

    pub fn row<T>(&mut self, dim: &DimensionReport<T>, time: DateTime<Tz>)
    where
        T: Into<V> + Mul<Money, Output = Money> + Copy,
    {
        self.rows.push(PeriodComponent {
            time,
            price: dim.price.as_ref().map(|p| p.price).into(),
            volume: dim.volume.map(Into::into).into(),
            billed_volume: dim.billed_volume.map(Into::into).into(),
            vat: dim.price.as_ref().and_then(|p| p.vat).into(),
            cost_excl_vat: dim.cost_excl_vat(),
            cost_incl_vat: dim.cost_incl_vat(),
        });
    }

    pub fn into_table(self) -> Table {
        let mut table = Table::new(self.rows);

        table
            .with(Style::modern())
            .with(Panel::header(style(self.name).bold().to_string()))
            .with(Modify::new(Segment::all()).with(MinWidth::new(10)))
            .with(Alignment::center());

        table
    }
}

#[derive(Tabled)]
pub struct PeriodComponent<V: Display> {
    #[tabled(rename = "Time", display_with = "format_time")]
    time: DateTime<Tz>,
    #[tabled(rename = "Price")]
    price: OptionDisplay<Money>,
    #[tabled(rename = "VAT")]
    vat: OptionDisplay<Vat>,
    #[tabled(rename = "Volume")]
    volume: OptionDisplay<V>,
    #[tabled(rename = "Billed volume")]
    billed_volume: OptionDisplay<V>,
    #[tabled(rename = "Cost excl. VAT")]
    cost_excl_vat: Money,
    #[tabled(rename = "Cost incl. VAT")]
    cost_incl_vat: Money,
}

fn format_time(time: &DateTime<Tz>) -> String {
    time.format("%y-%m-%d %H:%M:%S").to_string()
}

pub struct OptionDisplay<T>(Option<T>);

impl<T> From<Option<T>> for OptionDisplay<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<T> Display for OptionDisplay<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(value) = &self.0 {
            value.fmt(f)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy)]
pub struct UnitDisplay;

impl Display for UnitDisplay {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl From<()> for UnitDisplay {
    fn from(_: ()) -> Self {
        UnitDisplay
    }
}
