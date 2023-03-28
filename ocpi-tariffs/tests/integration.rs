use chrono::{Duration, TimeZone, Utc};
use ocpi_tariffs::ocpi::cdr::Cdr;
use rust_decimal_macros::dec;

mod common;

use common::validate_cdr;

#[test]
fn test_json_files() {
    let mut should_panic = false;

    for json_test in common::collect_json_tests().unwrap() {
        let tariff = json_test.tariff;

        eprintln!("\ntesting directory {:?}", json_test.path);

        for (name, cdr) in json_test.cdrs {
            eprint!("  testing json cdr `{}`: ", name);

            let result = std::panic::catch_unwind(|| {
                common::validate_cdr(cdr, tariff.clone()).unwrap();
            });

            if let Err(_) = result {
                should_panic = true;
            } else {
                eprintln!("success")
            }
        }
    }

    if should_panic {
        panic!("not all json tests succeeded")
    }
}
