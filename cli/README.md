
# OCPI tariffs CLI

This crate provides a binary for doing calculations with [OCPI](https://evroaming.org/ocpi-background/)
[tariffs](https://github.com/ocpi/ocpi/blob/2.2.1/mod_tariffs.asciidoc#1-tariffs-module).
Specifically for the [`OCPI 2.2.1`](https://evroaming.org/app/uploads/2021/11/OCPI-2.2.1.pdf) 
and [`OCPI 2.1.1`](https://github.com/ocpi/ocpi/releases/download/2.1.1-d2/OCPI_2.1.1-d2.pdf) version.

## Installation

First install [Rust](https://www.rust-lang.org/tools/install) and then install
the OCPI tariffs command line tool:

```text
cargo install ocpi-tariffs-cli
```

## Usage

The binary can be directly executed using `ocpi-tariffs`. Execute `ocpi-tariffs
--help` for a list of subcommands.

### Subcomamands

#### Analyze

To price a tariff and CDR (Charge detail record) and see a breakdown of the
separate periods use `analyze`:

```text
Analyze a given charge detail record (CDR) against either a provided tariff
structure or a tariff that is contained in the CDR itself.

This command will show you a breakdown of all the calculated costs.

Usage: ocpi-tariffs analyze [OPTIONS]

Options:
  -c, --cdr <CDR>
          A path to the charge detail record in json format.

          If no path is provided the CDR is read from standard in.

  -t, --tariff <TARIFF>
          A path to the tariff structure in json format.

          If no path is provided, then the tariff is inferred to be contained
          inside the provided CDR. If the CDR contains multiple tariff
          structures, the first valid tariff will be used.

  -o, --ocpi-version <OCPI_VERSION>
          The OCPI version that should be used for the input structures.

          If the input consists of version 2.1.1 structures they will be converted to 2.2.1 structures. 
          The actual calculation and output will always be according to OCPI 2.2.1.

          use `detect` to let to tool try to find the matching version.

          [default: v221]
          [possible values: v221, v211, detect]  


  -z, --timezone <TIMEZONE>
          Timezone for evaluating any local times contained in the tariff
          structure

          [default: Europe/Amsterdam]

  -h, --help
          Print help (see a summary with '-h')
```

#### Validate

To price a tariff and CDR and check if the calculation differs from the original
CDR use `validate`:

```text
Validate a given charge detail record (CDR) against either a provided tariff
structure or a tariff that is contained in the CDR itself.

This command will show the differences between the calculated totals and the
totals contained in the provided CDR.

Usage: ocpi-tariffs validate [OPTIONS]

Options:
  -c, --cdr <CDR>
          A path to the charge detail record in json format.

          If no path is provided the CDR is read from standard in.

  -t, --tariff <TARIFF>
          A path to the tariff structure in json format.

          If no path is provided, then the tariff is inferred to be contained
          inside the provided CDR. If the CDR contains multiple tariff
          structures, the first valid tariff will be used.

  -z, --timezone <TIMEZONE>
          Timezone for evaluating any local times contained in the tariff
          structure

          [default: Europe/Amsterdam]

  -o, --ocpi-version <OCPI_VERSION>
          The OCPI version that should be used for the input structures.

          If the input consists of version 2.1.1 structures they will be converted to 2.2.1 structures. 
          The actual calculation and output will always be according to OCPI 2.2.1.

          use `detect` to let to tool try to find the matching version.

          [default: v221]
          [possible values: v221, v211, detect]  

  -h, --help
          Print help (see a summary with '-h')
```
