use std::{
    borrow::Cow,
    fmt::Display,
    fs::File,
    io::{stdin, Read},
    iter,
    path::PathBuf,
    process::exit,
};

use chrono_tz::Tz;
use clap::{Args, Parser, Subcommand, ValueEnum};
use console::style;
use serde::de::DeserializeOwned;

use ocpi_tariffs::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff, v211},
    pricer::{Pricer, Report},
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
    #[arg(short = 'z', long, alias = "tz")]
    timezone: Option<Tz>,
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

        let mut pricer = Pricer::new(&cdr).detect_time_zone(true);

        if let Some(tariff) = &tariff {
            pricer = pricer.with_tariffs([tariff]);
        };

        if let Some(time_zone) = self.timezone {
            pricer = pricer.with_time_zone(time_zone);
        }

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

impl Validate {
    fn run(self) -> Result<()> {
        let (report, cdr, _) = self.args.load_all()?;

        println!(
            "\n{} `{}` with tariff `{}`, using timezone `{}`:",
            style("Validating").green().bold(),
            style(self.args.cdr_name()).blue(),
            style(self.args.tariff_name()).blue(),
            style(&report.time_zone).blue(),
        );

        let mut table = Table::new();
        let mut is_valid = false;

        table.header(&["Property", "Report", "Cdr"]);

        table.row(&[
            "Total Time".into(),
            report.total_time.to_string(),
            cdr.total_time.to_string(),
        ]);

        is_valid &= report.total_time == cdr.total_time;

        table.row(&[
            "Total Parking Time".into(),
            report.total_parking_time.to_string(),
            to_string_or_default(cdr.total_parking_time),
        ]);

        is_valid &= cdr
            .total_parking_time
            .map(|c| c == report.total_parking_time)
            .unwrap_or(true);

        table.row(&[
            "Total Energy".into(),
            report.total_energy.with_scale().to_string(),
            cdr.total_energy.to_string(),
        ]);

        is_valid &= report.total_energy == cdr.total_energy;

        table.row(&[
            "Total Cost (Excl.)".into(),
            to_string_or_default(report.total_cost.map(|p| p.excl_vat)),
            cdr.total_cost.with_scale().excl_vat.to_string(),
        ]);

        table.row(&[
            "Total Cost (Incl.)".into(),
            to_string_or_default(report.total_cost.and_then(|p| p.incl_vat)),
            to_string_or_default(cdr.total_cost.incl_vat),
        ]);

        is_valid &= report
            .total_cost
            .map(|p| p == cdr.total_cost)
            .unwrap_or(true);

        table.row(&[
            "Total Time Cost (Excl.)".into(),
            to_string_or_default(report.total_time_cost.map(|p| p.with_scale().excl_vat)),
            to_string_or_default(cdr.total_time_cost.map(|p| p.excl_vat)),
        ]);

        table.row(&[
            "Total Time Cost (Incl.)".into(),
            to_string_or_default(report.total_time_cost.and_then(|p| p.with_scale().incl_vat)),
            to_string_or_default(cdr.total_time_cost.and_then(|p| p.incl_vat)),
        ]);

        is_valid &= report
            .total_time_cost
            .zip(cdr.total_time_cost)
            .map(|(l, r)| l == r)
            .unwrap_or(true);

        table.row(&[
            "Total Fixed Cost (Excl.)".into(),
            to_string_or_default(report.total_fixed_cost.map(|p| p.excl_vat)),
            to_string_or_default(cdr.total_fixed_cost.map(|p| p.excl_vat)),
        ]);

        table.row(&[
            "Total Fixed Cost (Incl.)".into(),
            to_string_or_default(report.total_fixed_cost.and_then(|p| p.incl_vat)),
            to_string_or_default(cdr.total_fixed_cost.and_then(|p| p.incl_vat)),
        ]);

        is_valid &= report
            .total_fixed_cost
            .zip(cdr.total_fixed_cost)
            .map(|(l, r)| l == r)
            .unwrap_or(true);

        table.row(&[
            "Total Energy Cost (Excl.)".into(),
            to_string_or_default(report.total_energy_cost.map(|p| p.excl_vat)),
            to_string_or_default(cdr.total_energy_cost.map(|p| p.excl_vat)),
        ]);

        table.row(&[
            "Total Energy Cost (Incl.)".into(),
            to_string_or_default(report.total_energy_cost.and_then(|p| p.incl_vat)),
            to_string_or_default(cdr.total_energy_cost.and_then(|p| p.incl_vat)),
        ]);

        is_valid &= report
            .total_energy_cost
            .zip(cdr.total_energy_cost)
            .map(|(l, r)| l == r)
            .unwrap_or(true);

        table.row(&[
            "Total Parking Cost (Excl.)".into(),
            to_string_or_default(report.total_parking_cost.map(|p| p.excl_vat)),
            to_string_or_default(cdr.total_parking_cost.map(|p| p.excl_vat)),
        ]);

        table.row(&[
            "Total Parking Cost (Incl.)".into(),
            to_string_or_default(report.total_parking_cost.and_then(|p| p.incl_vat)),
            to_string_or_default(cdr.total_parking_cost.and_then(|p| p.incl_vat)),
        ]);

        is_valid &= report
            .total_parking_cost
            .zip(cdr.total_parking_cost)
            .map(|(l, r)| l == r)
            .unwrap_or(true);

        table.retain_rows(|v| !v[1].is_empty() || !v[2].is_empty());

        println!("{}", table);

        if !is_valid {
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
            style(&report.time_zone).blue(),
        );

        let time_zone: Tz = report.time_zone.parse().expect("invalid time zone");

        let mut table = Table::new();

        table.header(&[
            "Period",
            "",
            "Energy",
            "Charging Time",
            "Parking Time",
            "Flat",
        ]);

        for period in report.periods.iter() {
            let start_time = period.start_date_time.with_timezone(&time_zone);
            let dim = &period.dimensions;

            table.row(&[
                start_time.to_string(),
                "Volume".to_string(),
                to_string_or_default(dim.energy.volume),
                to_string_or_default(dim.time.volume),
                to_string_or_default(dim.parking_time.volume),
                dim.flat.price.map(|_| "x".to_string()).unwrap_or_default(),
            ]);

            table.row(&[
                "".to_string(),
                "Price".to_string(),
                to_string_or_default(dim.energy.price.map(|p| p.price)),
                to_string_or_default(dim.time.price.map(|p| p.price)),
                to_string_or_default(dim.parking_time.price.map(|p| p.price)),
                to_string_or_default(dim.flat.price.map(|p| p.price)),
            ]);
        }

        table.line();

        table.row(&[
            "Total".to_string(),
            "Volume".to_string(),
            report.total_energy.to_string(),
            report.total_time.to_string(),
            report.total_parking_time.to_string(),
            "".to_string(),
        ]);

        table.row(&[
            "".to_string(),
            "Price".to_string(),
            to_string_or_default(report.total_energy_cost.map(|p| p.excl_vat)),
            to_string_or_default(report.total_time_cost.map(|p| p.excl_vat)),
            to_string_or_default(report.total_parking_cost.map(|p| p.excl_vat)),
            to_string_or_default(report.total_fixed_cost.map(|p| p.excl_vat)),
        ]);

        println!("{}", table);

        Ok(())
    }
}

struct Table {
    widths: Vec<usize>,
    items: Vec<Item>,
}

impl Table {
    fn new() -> Self {
        Self {
            widths: Vec::new(),
            items: Vec::new(),
        }
    }

    fn retain_rows(&mut self, func: impl Fn(&[String]) -> bool) {
        self.items.retain(|v| {
            if let Item::Row(row) = v {
                func(row)
            } else {
                true
            }
        });
    }

    fn line(&mut self) {
        self.items.push(Item::Line);
    }

    fn row<R>(&mut self, row: R)
    where
        R: IntoIterator,
        R::Item: Display,
    {
        let mut values = Vec::new();

        for (i, value) in row.into_iter().enumerate() {
            let value = value.to_string();

            if i == self.widths.len() {
                self.widths.push(value.len());
            } else {
                self.widths[i] = self.widths[i].max(value.len());
            }

            values.push(value);
        }

        self.items.push(Item::Row(values));
    }

    fn header<H>(&mut self, header: H)
    where
        H: IntoIterator,
        H::Item: Display,
    {
        self.items.push(Item::Line);
        self.row(header);
        self.items.push(Item::Line);
    }
}

impl Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut line = false;

        let iter = iter::once(&Item::Line)
            .chain(&self.items)
            .chain(iter::once(&Item::Line));

        for item in iter {
            match item {
                Item::Line if !line => {
                    write!(f, "+")?;

                    for width in &self.widths {
                        write!(f, "{0:->1$}+", "", width + 2)?;
                    }

                    writeln!(f)?;

                    line = true;
                }
                Item::Row(row) => {
                    write!(f, "|")?;

                    for (value, &width) in row.iter().zip(&self.widths) {
                        write!(f, " {0: <1$} |", value, width)?;
                    }

                    writeln!(f)?;

                    line = false;
                }
                Item::Line => {}
            }
        }

        Ok(())
    }
}

enum Item {
    Row(Vec<String>),
    Line,
}

fn to_string_or_default<T: Display>(v: Option<T>) -> String {
    v.map(|v| v.to_string()).unwrap_or_default()
}
