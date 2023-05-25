To publish the crates follow the following steps:

1. Update the version number in the top level `Cargo.toml` and use the same
   version number for the `ocpi-tariffs` dependency in `cli/Cargo.toml`. Update
   the `CHANGELOG.md` Have this reviewed and merged into the main branch.

2. Once that's merged tag the right commit on the main branch and push this tag
   to GitHub.

3. Publish the `ocpi-tariffs` crate **first**.

4. Then publish the `cli` crate (because it depends on the `ocpi-tariffs`
   crate).

Enjoy the new crates!
