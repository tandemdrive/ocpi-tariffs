mod ocpi;

pub use ocpi::cdr::Cdr;




pub fn calculate(cdr: &Cdr) {
    for period in cdr.charging_periods.iter() {

    }
}
