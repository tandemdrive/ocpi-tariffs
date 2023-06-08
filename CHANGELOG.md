# Changelog

All notable changes to this project will be documented in this file.

## 0.4.0 2023-06-07

Changed serialization of all structures to use OCPI rounding.

Added library support for OCPI 2.1.1 that can be enabled with feature flag `ocpi-v211`.

Added cli support for OCPI 2.1.1 that can be used by specifying `--ocpi-version v211` or `--ocpi-version detect`.

## 0.3.2 2023-05-30

Fixed a bug in step-size calculation for energy dimension.

Fixed a bug where step-sizes where not affecting the period `billed_volume`.

## 0.3.1 2023-05-25

Renamed `stop_date_time` to `end_date_time` as specified by OCPI 2.2.1.
