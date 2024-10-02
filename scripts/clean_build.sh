#!/bin/sh

set -e

start=$(date -Iseconds -u)
host_name=$(hostname)
echo "Starting build at: ${start} on ${host_name}"

for dir in "${BUILD_OUTPUT}/debug/build" \
    "${BUILD_OUTPUT}/debug/deps" \
    "${BUILD_OUTPUT}/debug/incremental"
do
    if [ -d "${dir}" ] ; then
        find "${dir}" ! -newerct '15 days ago' -d -exec rm -rv {} \;
    fi
done
