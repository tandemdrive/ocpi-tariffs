# OCPI tariffs

[![crates-io]](https://crates.io/crates/ocpi-tariffs "Crates.io version")
[![docs-rs]](https://docs.rs/ocpi-tariffs "Documentation")
[![unsafe-forbidden]](https://github.com/rust-secure-code/safety-dance/ "Unsafe forbidden")
[![github-actions]](https://github.com/tandemdrive/ocpi-tariffs/actions "CI")
[![github-activity]](https://github.com/tandemdrive/ocpi-tariffs/commits "Commit activity")

[crates-io]: https://img.shields.io/crates/v/ocpi-tariffs.svg?logo=rust
[docs-rs]: https://img.shields.io/docsrs/ocpi-tariffs/latest?logo=docs.rs
[unsafe-forbidden]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[github-actions]: https://img.shields.io/github/actions/workflow/status/tandemdrive/ocpi-tariffs/rust.yml?branch=main
[github-activity]: https://img.shields.io/github/last-commit/tandemdrive/ocpi-tariffs
[ocpi]: https://evroaming.org/ocpi-background/
[tariffs]: https://github.com/ocpi/ocpi/blob/2.2.1/mod_tariffs.asciidoc#1-tariffs-module

This project provides software for doing calculations with [OCPI](https://evroaming.org/ocpi-background/)
[tariffs](https://github.com/ocpi/ocpi/blob/2.2.1/mod_tariffs.asciidoc#1-tariffs-module).
Specifically for the [`OCPI 2.2.1`](https://evroaming.org/app/uploads/2021/11/OCPI-2.2.1.pdf)
and [`OCPI 2.1.1`](https://github.com/ocpi/ocpi/releases/download/2.1.1-d2/OCPI_2.1.1-d2.pdf) version.

You can test the tariff tool [online](https://tools.tandemdrive.com/).

OCPI is a protocol owned and maintained by the [EV Roaming
foundation](https://evroaming.org/).

## Goals

- Provide a reference OCPI tariff implementation

  Given a certain
  [CDR](https://github.com/ocpi/ocpi/blob/2.2.1/mod_cdrs.asciidoc) and
  a certain tariff it's able to calculate the different (sub)totals.

  The software provides as output a breakdown of how these (sub)totals were
  calculated.

- Provide an extensive set of test-cases to ensure correctness.

- Collect feedback from the community

  We aim to have as many parties as possible agree that this reference
  implementation is a correct interpretation of the OCPI specification and
  intentions. We aim for close cooperation with the [EV Roaming
  Foundation](https://evroaming.org/).

  We welcome community contributions to this project. We aim to make it easy for
  people, familiar with OCPI, to provide new test cases.

- Provide the software as open source software so others can freely use and
  test it.

  Both a command line tool and a Rust library are provided.

- Expose the software functionality [online](https://tools.tandemdrive.com/)
  to make it even easier for people to do calculations with the tariff
  software.

- Support OCPI 3 tariffs soon after OCPI 3 is finalized.

## Non-goals

- Provide software which communicates tariffs using the OCPI protocol between
  EV roaming parties.

- Produce human consumable receipts which specify the different cost components
  that led to the calculated total price.

## Tariff structure overview

```mermaid
classDiagram
    class Tariff {
        TariffElement elements
    }
    class TariffElement {
        PriceComponent price_components
        TariffRestriction restrictions
    }
    class PriceComponent {
        TarifDimensionType type
        Number price
        Number vat
        int step_size
    }
    class TariffRestriction {
        DayOfWeek day_of_week
        ...
    }
    Tariff "1" --o "1.." TariffElement
    TariffElement "1" --o "1.." PriceComponent
    TariffElement "1" --o "0..1" TariffRestriction
```

## Interpretation

This implementation aims to follow the OCPI specification as closely as 
possible. However, as with any specification, details might be left open to 
interpretation. The following is a list of assumptions that have been made in 
this implementation:

- We assume that a `FLAT` price component can only be active once in a session.
  The first time a `FLAT` price component becomes active it will be used and 
  subsequent active `FLAT` components will be ignored. Although this is not 
  explicitly mentioned in the OCPI 2.*.* specs, we feel it is the correct 
  interpretation.

- We assume that the charging periods provided as input are well-formed. 
  Meaning, each period in the session that has a different price must be 
  provided as a separate charging period in the CDR. No attempt will be made
  to subdivide or interpolate data inside a single provided period.

## Contributing

We welcome community contributions to this project.

Please read our [Contributor Terms](CONTRIBUTING.md#contributor-terms) before
you make any contributions.

Any contribution intentionally submitted for inclusion, shall comply with the
Rust standard licensing model (MIT OR Apache 2.0) and therefore be dual licensed
as described below, without any additional terms or conditions:

### License

This contribution is dual licensed under EITHER OF

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

For clarity, "your" refers to TandemDrive or any other licensee/user of the contribution.
