use std::{borrow::Cow, fmt::Display, fs::File, io::stdin, path::PathBuf, process::exit};

use chrono_tz::Tz;
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use ocpi_tariffs::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff},
    pricer::{Pricer, Report},
    types::money::Price,
};
use tabled::{Style, Table, Tabled};

use crate::{error::Error, Result};

#[derive(Debug, Parser)]
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

#[derive(Debug, Subcommand)]
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

#[derive(Debug, Args)]
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
    fn cdr_name(&self) -> Cow<str> {
        self.cdr.as_ref().map_or("<stdin>".into(), |c| {
            c.file_name().unwrap().to_string_lossy()
        })
    }

    fn tariff_name(&self) -> Cow<str> {
        self.tariff.as_ref().map_or("<CDR-tariff>".into(), |c| {
            c.file_name().unwrap().to_string_lossy()
        })
    }

    fn load_all(&self) -> Result<(Report, Cdr, Option<OcpiTariff>)> {
        let cdr: Cdr = if let Some(cdr_path) = &self.cdr {
            let file = File::open(&cdr_path).map_err(|e| Error::file(cdr_path.clone(), e))?;
            serde_json::from_reader(&file)
                .map_err(|e| Error::deserialize(cdr_path.display(), "CDR", e))?
        } else {
            let mut stdin = stdin().lock();
            serde_json::from_reader(&mut stdin)
                .map_err(|e| Error::deserialize("<stdin>", "CDR", e))?
        };

        let tariff: Option<OcpiTariff> = if let Some(path) = &self.tariff {
            let file = File::open(&path).map_err(|e| Error::file(path.clone(), e))?;
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

#[derive(Debug, Parser)]
pub struct Validate {
    #[command(flatten)]
    args: TariffArgs,
}

#[derive(Debug, Tabled)]
struct ValidateRow {
    name: String,
    report: String,
    #[tabled(rename = "CDR")]
    cdr: String,
}

impl ValidateRow {
    fn error(self) -> Self {
        Self {
            name: self.name.red().to_string(),
            report: self.report.red().to_string(),
            cdr: self.cdr.red().to_string(),
        }
    }
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
        let error = report == cdr.clone().unwrap_or_default();
        let row = ValidateRow {
            name: name.to_string(),
            report: report.to_string(),
            cdr: cdr.map_or("<missing>".into(), |s| s.to_string()),
        };

        if error {
            self.rows.push(row.error())
        } else {
            self.rows.push(row)
        }
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
}

impl Validate {
    fn run(self) -> Result<()> {
        let (report, cdr, _) = self.args.load_all()?;

        println!(
            "{} `{}` with tariff `{}`",
            "Validating".green(),
            self.args.cdr_name(),
            self.args.tariff_name()
        );

        let mut table = ValidateTable { rows: Vec::new() };

        table.row(report.total_time.into(), Some(cdr.total_time), "Total Time");
        table.row(
            report.total_parking_time.into(),
            cdr.total_parking_time,
            "Total Parking time",
        );
        table.row(report.total_energy, Some(cdr.total_energy), "Total Energy");

        table.price_row(report.total_cost, Some(cdr.total_cost), "Total Cost");

        table.price_row(
            report.total_time_cost,
            cdr.total_time_cost,
            "Total Time cost",
        );
        table.price_row(
            report.total_fixed_cost,
            cdr.total_fixed_cost,
            "Total Fixed cost",
        );
        table.price_row(
            report.total_energy_cost,
            cdr.total_energy_cost,
            "Total Energy cost",
        );
        table.price_row(
            report.total_parking_cost,
            cdr.total_parking_cost,
            "Total Parking cost",
        );

        println!(
            "{}",
            Table::new(table.rows).with(Style::modern()).to_string()
        );

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct Analyze {
    #[command(flatten)]
    args: TariffArgs,
}

impl Analyze {
    fn run(self) -> Result<()> {
        let (report, _, _) = self.args.load_all()?;

        Ok(())
    }
}
