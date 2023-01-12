# OCPI tariffs

This crate implements a price calculator for [OCPI][ocpi] [tariffs][tariffs]. Specifically for the [`OCPI 2.2.1`](https://evroaming.org/app/uploads/2021/11/OCPI-2.2.1.pdf) version. Note that the OCPI specification v2 development occurs at [GitHub][ocpi-gh]. 

## Goals

- Provide a reference implementation of a price calculator for [charge session]() given a set of tariffs.

## Non-goals

- Implement an HTTP API conforming to the OCPI specification.
- Implement a directly human consumable invoice specifying the different cost components leading to the calculated total price.

## Tariff structure overview

``` mermaid
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
    }
    Tariff "1" --o "+" TariffElement
    TariffElement "1" --o "1.." PriceComponent
    TariffElement "1" --o "0..1" TariffRestriction
```


[ocpi]: https://evroaming.org/ocpi-background/
[ocpi-gh]: https://github.com/ocpi/ocpi
[tariffs]: https://github.com/ocpi/ocpi/blob/2.2.1/mod_tariffs.asciidoc#1-tariffs-module
