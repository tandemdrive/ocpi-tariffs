use clap::Parser;

fn main() {
    ocpi_tariffs_cli::Cli::parse().run();
}
