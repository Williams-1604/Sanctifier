# Contributing to Sanctifier

Thanks for helping improve Sanctifier.

## Required repository secrets

### `CRATES_IO_TOKEN`

The release workflow publishes both `sanctifier-core` and `sanctifier-cli`
when a version tag such as `v0.1.0` is pushed.

### `CODECOV_TOKEN`

The CI workflow uploads the `cobertura.xml` coverage report to Codecov by using
`codecov/codecov-action@v4.6.0` pinned to an immutable commit SHA.

## To configure the tokens:

1. Open the Sanctifier repository on GitHub.
2. Go to `Settings` -> `Secrets and variables` -> `Actions`.

### For publishing crates:
3. Create a new repository secret named `CRATES_IO_TOKEN`.
4. Generate the token from your https://crates.io/settings/tokens.

The workflow publishes `sanctifier-core` first and then `sanctifier-cli`,
because the CLI depends on the core crate.

### For coverage reporting:
3. Create a new repository secret named `CODECOV_TOKEN`.
4. Copy the upload token for `HyperSafeD/Sanctifier` from https://codecov.io/.

Without this secret, the coverage upload step will fail to authenticate.