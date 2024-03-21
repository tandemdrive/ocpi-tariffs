use std::{
    borrow::Cow,
    fmt::Display,
    fs::File,
    io::{stdin, Read},
    path::PathBuf,
    process::exit,
};

use chrono::DateTime;
use chrono_tz::Tz;
use clap::{Args, Parser, Subcommand, ValueEnum};
use console::style;
use serde::de::DeserializeOwned;
use tabled::{
    settings::{
        object::{Rows, Segment},
        width::MinWidth,
        Alignment, Format, Modify, Panel, Style,
    },
    Table, Tabled,
};

use ocpi_tariffs::{
    ocpi::{
        cdr::Cdr,
        tariff::{CompatibilityVat, OcpiTariff},
        v211,
    },
    pricer::{Dimension, DimensionReport, Pricer, Report},
    types::{
        electricity::Kwh,
        money::{Money, Price},
        time::HoursDecimal,
    },
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
            eprintln!("{err}");
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
    /// The OCPI version that should be used for the input structures.
    ///
    /// If the input consists of version 2.1.1 structures they will be converted to 2.2.1
    /// structures. The actual calculation and output will always be according to OCPI 2.2.1.
    ///
    /// use `detect` to let to tool try to find the matching version.
    #[arg(short = 'o', long, value_enum, default_value_t = OcpiVersion::default())]
    ocpi_version: OcpiVersion,
}

impl TariffArgs {
    fn cdr_name(&self) -> Cow<'_, str> {
        self.cdr
            .as_ref()
            .map(|c| c.file_name().unwrap().to_string_lossy())
            .unwrap_or_else(|| "<stdin>".into())
    }

    fn tariff_name(&self) -> Cow<'_, str> {
        self.tariff
            .as_ref()
            .map(|c| c.file_name().unwrap().to_string_lossy())
            .unwrap_or_else(|| "<CDR-tariff>".into())
    }

    fn load_all(&self) -> Result<(Report, Cdr, Option<OcpiTariff>)> {
        let cdr: Cdr = if let Some(cdr_path) = &self.cdr {
            let file = File::open(cdr_path).map_err(|e| Error::file(cdr_path.clone(), e))?;

            from_reader_with_version::<_, _, v211::cdr::Cdr>(file, self.ocpi_version)
                .map_err(|e| Error::deserialize(cdr_path.display(), "CDR", e))?
        } else {
            let mut stdin = stdin().lock();
            from_reader_with_version::<_, _, v211::cdr::Cdr>(&mut stdin, self.ocpi_version)
                .map_err(|e| Error::deserialize("<stdin>", "CDR", e))?
        };

        let tariff: Option<OcpiTariff> = if let Some(path) = &self.tariff {
            let file = File::open(path).map_err(|e| Error::file(path.clone(), e))?;

            Some(
                from_reader_with_version::<_, _, v211::tariff::OcpiTariff>(file, self.ocpi_version)
                    .map_err(|e| Error::deserialize(path.display(), "tariff", e))?,
            )
        } else {
            None
        };

        let mut pricer = Pricer::new(&cdr);

        if let Some(tariff) = &tariff {
            pricer = pricer.with_tariffs([tariff]);
        };

        let report = pricer.build_report().map_err(Error::Internal)?;

        Ok((report, cdr, tariff))
    }
}

pub fn from_reader_with_version<R, T0, T1>(
    mut reader: R,
    version: OcpiVersion,
) -> std::io::Result<T0>
where
    R: Read,
    T0: DeserializeOwned + From<T1>,
    T1: DeserializeOwned,
{
    match version {
        OcpiVersion::V221 => Ok(serde_json::from_reader::<R, T0>(reader)?),
        OcpiVersion::V211 => Ok(serde_json::from_reader::<R, T1>(reader)?.into()),
        OcpiVersion::Detect => {
            let mut content = Vec::new();
            reader.read_to_end(&mut content)?;

            serde_json::from_slice::<T0>(&content).or_else(|err| {
                Ok(serde_json::from_slice::<T1>(&content)
                    .map_err(|_old_err| err)?
                    .into())
            })
        }
    }
}

#[derive(Clone, Copy, Default, ValueEnum)]
pub enum OcpiVersion {
    V221,
    V211,
    #[default]
    Detect,
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
    #[tabled(rename = "Validity")]
    validity: Validity,
}

#[derive(Clone, Copy)]
pub enum Validity {
    Valid,
    Invalid,
    Unknown,
}

impl Display for Validity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
            Self::Unknown => "unknown",
        })
    }
}

struct ValidateTable {
    rows: Vec<ValidateRow>,
}

impl ValidateTable {
    pub fn row<T: Display + Eq + Default + Clone>(
        &mut self,
        report: Option<T>,
        cdr: Option<T>,
        name: &str,
    ) {
        let validity = if let (Some(cdr), Some(report)) = (&cdr, &report) {
            if cdr == report {
                Validity::Valid
            } else {
                Validity::Invalid
            }
        } else {
            Validity::Unknown
        };

        let row = ValidateRow {
            property: name.to_string(),
            calculated: report
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<missing>".into()),
            cdr: cdr
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<missing>".into()),
            validity,
        };

        self.rows.push(row);
    }

    pub fn price_row(&mut self, report: Price, cdr: Option<Price>, name: &str) {
        self.row(
            Some(report.excl_vat),
            cdr.map(|s| s.excl_vat),
            &format!("{name} excl. VAT"),
        );
        self.row(
            report.incl_vat,
            cdr.and_then(|s| s.incl_vat),
            &format!("{name} incl. VAT"),
        );
    }

    pub fn valid_rows(&self) -> Vec<Validity> {
        self.rows.iter().map(|r| r.validity).collect()
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

        table.row(Some(report.total_time), Some(cdr.total_time), "Total Time");
        table.row(
            Some(report.total_parking_time),
            cdr.total_parking_time,
            "Total Parking time",
        );
        table.row(
            Some(report.total_energy.with_scale()),
            Some(cdr.total_energy),
            "Total Energy",
        );

        table.price_row(
            report.total_cost.unwrap_or_default().with_scale(),
            Some(cdr.total_cost),
            "Total Cost",
        );

        table.price_row(
            report.total_time_cost.unwrap_or_default().with_scale(),
            cdr.total_time_cost,
            "Total Time cost",
        );
        table.price_row(
            report.total_fixed_cost.unwrap_or_default().with_scale(),
            cdr.total_fixed_cost,
            "Total Fixed cost",
        );
        table.price_row(
            report.total_energy_cost.unwrap_or_default().with_scale(),
            cdr.total_energy_cost,
            "Total Energy cost",
        );
        table.price_row(
            report.total_parking_cost.unwrap_or_default().with_scale(),
            cdr.total_parking_cost,
            "Total Parking cost",
        );

        let valid = table.valid_rows();
        let is_invalid = valid.iter().any(|&s| matches!(s, Validity::Invalid));

        let format_valid = Modify::new(Rows::new(..)).with(Format::positioned(|row, (i, _)| {
            let row = style(row);
            if i == 0 {
                row.bold().to_string()
            } else if let Some(valid) = valid.get(i - 1) {
                match valid {
                    Validity::Valid => row.green().to_string(),
                    Validity::Unknown => row.yellow().to_string(),
                    Validity::Invalid => row.red().to_string(),
                }
            } else {
                row.to_string()
            }
        }));

        println!(
            "{}",
            Table::new(table.rows)
                .with(Style::modern())
                .with(format_valid)
        );

        if is_invalid {
            println!(
                "Calculation {} all totals in the CDR.\n",
                style("does not match").red().bold()
            );

            exit(1);
        } else {
            println!(
                "Calculation {} all totals in the CDR.\n",
                style("matches").green().bold()
            );
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
        T: Into<V> + Dimension,
    {
        let cost = dim.cost();
        self.rows.push(PeriodComponent {
            time,
            price: dim.price.as_ref().map(|p| p.price).into(),
            volume: dim.volume.map(Into::into).into(),
            billed_volume: dim.billed_volume.map(Into::into).into(),
            vat: dim.price.as_ref().map(|p| p.vat),
            cost_excl_vat: cost.map(|c| c.excl_vat).into(),
            cost_incl_vat: cost
                .map(|c| {
                    c.incl_vat
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| "<unknown>".into())
                })
                .into(),
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
    #[tabled(rename = "VAT", display_with = "format_vat")]
    vat: Option<CompatibilityVat>,
    #[tabled(rename = "Volume")]
    volume: OptionDisplay<V>,
    #[tabled(rename = "Billed volume")]
    billed_volume: OptionDisplay<V>,
    #[tabled(rename = "Cost excl. VAT")]
    cost_excl_vat: OptionDisplay<Money>,
    #[tabled(rename = "Cost incl. VAT")]
    cost_incl_vat: OptionDisplay<String>,
}

fn format_time(time: &DateTime<Tz>) -> String {
    time.format("%y-%m-%d %H:%M:%S").to_string()
}

fn format_vat(vat: &Option<CompatibilityVat>) -> String {
    if let Some(vat) = *vat {
        match vat {
            CompatibilityVat::Vat(vat) => OptionDisplay(vat).to_string(),
            CompatibilityVat::Unknown => "<unknown>".into(),
        }
    } else {
        String::new()
    }
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
