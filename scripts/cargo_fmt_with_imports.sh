#!/bin/sh

set -e

cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=Crate
