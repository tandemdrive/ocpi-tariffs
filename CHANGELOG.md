# Changelog

All notable changes to this project will be documented in this file.

## 0.6.1 2024-08-20

- Fixed bug where multiple flat fees are generated during the session. Instead
  we now only use the first time a flat fee becomes active.

## 0.6.0 2024-06-04

- Upgrade dependencies.
- Improved pricer interface.
- Added time zone detection feature.

## 0.5.2 2024-03-12

- Upgrade dependencies
- Added error variant for overflows

## 0.5.1 2023-11-16

- Added the Cargo.lock file to the git repo
  See: https://blog.rust-lang.org/2023/08/29/committing-lockfiles.html
- Upgrade dependencies
- Added conversion from ocpi types to decimal

## 0.5.0 2023-07-10

Changed the output dimension report generics to use `Dimension` trait instead of `std::ops::Mul`. 

Fixed a bug were restriction's like `min_duration` and `max_duration` did not work in conjunction with `PARKING_TIME` dimensions.

Fixed a bug were a tariff with a `step_size` of zero would cause a panic.

Changed all non-dividing arithmetic to saturating operations.

Fixed a bug were a missing `tariffs` key in a `Cdr` would cause de-serialization errors.

Changed the testing infrastructure to embed the test json's into the binary.

## 0.4.0 2023-06-07

Changed serialization of all structures to use OCPI rounding.

Added library support for OCPI 2.1.1 that can be enabled with feature flag `ocpi-v211`.

Added cli support for OCPI 2.1.1 that can be used by specifying `--ocpi-version v211` or `--ocpi-version detect`.

## 0.3.2 2023-05-30

Fixed a bug in step-size calculation for energy dimension.

Fixed a bug where step-sizes where not affecting the period `billed_volume`.

## 0.3.1 2023-05-25

Renamed `stop_date_time` to `end_date_time` as specified by OCPI 2.2.1.
