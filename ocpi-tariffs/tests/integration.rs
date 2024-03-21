use chrono_tz::Tz;
use ocpi_tariffs::{
    ocpi::{cdr::Cdr, tariff::OcpiTariff},
    pricer::Pricer,
};
use std::path::PathBuf;

#[test_each::file(glob = "ocpi-tariffs/test_data/*/cdr*.json", name(segments = 2))]
fn test_json(cdr: &str, path: PathBuf) {
    let tariff = std::fs::read_to_string(path.parent().unwrap().join("tariff.json")).unwrap();

    let cdr = serde_json::from_str(cdr).unwrap();
    let tariff = serde_json::from_str(&tariff).unwrap();

    validate_cdr(&cdr, tariff).unwrap();
}

pub fn validate_cdr(cdr: &Cdr, tariff: OcpiTariff) -> Result<(), ocpi_tariffs::Error> {
    let report = Pricer::new(cdr)
        .with_tariffs(&[tariff])
        .with_time_zone(Tz::UTC)
        .build_report()?;

    assert_eq!(
        cdr.total_cost,
        report.total_cost.unwrap_or_default().with_scale(),
        "total_cost"
    );

    assert_eq!(
        cdr.total_energy,
        report.total_energy.with_scale(),
        "total_energy"
    );
    assert_eq!(
        cdr.total_energy_cost.unwrap_or_default(),
        report.total_energy_cost.unwrap_or_default().with_scale(),
        "total_energy_cost"
    );

    assert_eq!(cdr.total_time, report.total_time, "total_time");

    assert_eq!(
        cdr.total_time_cost.unwrap_or_default(),
        report.total_time_cost.unwrap_or_default().with_scale(),
        "total_time_cost"
    );

    assert_eq!(
        cdr.total_parking_time.unwrap_or_default(),
        report.total_parking_time,
        "total_parking_time"
    );

    assert_eq!(
        cdr.total_parking_cost.unwrap_or_default(),
        report.total_parking_cost.unwrap_or_default().with_scale(),
        "total_parking_cost"
    );

    assert_eq!(
        cdr.total_reservation_cost.unwrap_or_default(),
        report
            .total_reservation_cost
            .unwrap_or_default()
            .with_scale(),
        "total_reservation_cost"
    );

    assert_eq!(
        cdr.total_fixed_cost.unwrap_or_default(),
        report.total_fixed_cost.unwrap_or_default().with_scale(),
        "total_fixed_cost"
    );

    Ok(())
}
