mod common;

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

            if result.is_err() {
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
