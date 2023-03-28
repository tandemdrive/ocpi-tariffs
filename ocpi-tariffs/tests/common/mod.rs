use std::{
    fs::{read_dir, File},
    path::PathBuf,
};

use chrono::Duration;
use chrono_tz::Tz;
use ocpi_tariffs::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff},
    pricer::Pricer,
};

pub struct JsonTest {
    pub path: PathBuf,
    pub tariff: OcpiTariff,
    pub cdrs: Vec<(String, Cdr)>,
}

pub fn collect_json_tests() -> Result<Vec<JsonTest>, Box<dyn std::error::Error>> {
    let mut tests = Vec::new();

    for test_dir in read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/resources"))? {
        let test_dir_path = test_dir?.path();

        if !test_dir_path.is_dir() {
            continue;
        }

        let mut tariff = None;
        let mut cdrs = Vec::new();

        for json_file in read_dir(&test_dir_path)? {
            let file_path = json_file?.path();

            if file_path.extension().unwrap() != "json" {
                continue;
            }

            let file_stem = file_path.file_stem().unwrap();
            if file_stem == "tariff" {
                tariff = Some(serde_json::from_reader(File::open(file_path)?)?);
            } else {
                cdrs.push((
                    file_stem.to_string_lossy().to_string(),
                    serde_json::from_reader(File::open(file_path)?)?,
                ));
            }
        }

        tests.push(JsonTest {
            tariff: tariff
                .unwrap_or_else(|| panic!("no tariff.json in test directory {:?}", test_dir_path)),
            cdrs,
            path: test_dir_path,
        });
    }

    Ok(tests)
}

#[macro_export]
macro_rules! tariff {
    ($name:literal) => {
        serde_json::from_str::<'_, ocpi_tariffs::ocpi::tariff::OcpiTariff>(include_str!(concat!(
            "../resources/",
            $name,
            "/tariff.json"
        )))
        .unwrap()
    };
}

pub fn validate_cdr(cdr: Cdr, tariff: OcpiTariff) -> Result<(), ocpi_tariffs::Error> {
    let pricer = Pricer::with_tariffs(&cdr, &[tariff], Tz::UTC);
    let report = pricer.build_report()?;

    assert_eq!(cdr.total_cost, report.total_cost.with_scale(), "total_cost");

    assert_eq!(
        cdr.total_energy,
        report.total_energy.with_scale(),
        "total_energy"
    );
    assert_eq!(
        cdr.total_energy_cost.unwrap_or_default(),
        report.total_energy_cost.with_scale(),
        "total_energy_cost"
    );

    assert_eq!(
        Duration::from(cdr.total_time),
        report.total_time,
        "total_time"
    );

    assert_eq!(
        cdr.total_time_cost.unwrap_or_default(),
        report.total_time_cost.with_scale(),
        "total_time_cost"
    );

    assert_eq!(
        Duration::from(cdr.total_parking_time.unwrap_or_default()),
        report.total_parking_time,
        "total_parking_time"
    );

    assert_eq!(
        cdr.total_parking_cost.unwrap_or_default(),
        report.total_parking_cost.with_scale(),
        "total_parking_cost"
    );

    assert_eq!(
        cdr.total_reservation_cost.unwrap_or_default(),
        report.total_reservation_cost.with_scale(),
        "total_reservation_cost"
    );

    assert_eq!(
        cdr.total_fixed_cost.unwrap_or_default(),
        report.total_fixed_cost.with_scale(),
        "total_fixed_cost"
    );

    Ok(())
}
