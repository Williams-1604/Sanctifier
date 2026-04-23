# GitHub Action Support Matrix

This document describes the supported environment for the composite GitHub Action in `action.yml`.

## Supported runners

- **ubuntu-latest**: supported
- **macos-latest**: supported
- **windows-latest**: supported (best-effort; ensure the CLI can be installed for your target environment)

## Required tools on the runner

The action installs the Sanctifier CLI via `cargo install`, so the runner needs:

- **Rust toolchain** (via `rustup` / `cargo` in PATH)
- **Network access** to download Rust crates

If you use Sanctifier’s formal-verification features, your project may also require:

- **Z3** headers/libraries (platform-specific)

## GitHub token permissions

The action itself does not require elevated permissions to *run* a scan.

- **Minimum** (scan only): `contents: read`
- **SARIF upload enabled** (`format: sarif` + `upload-sarif: "true"`): `security-events: write`

Notes:

- The action intentionally skips SARIF upload on `pull_request` events originating from forks.
- If you disable SARIF upload (`upload-sarif: "false"`), you should omit `security-events: write`.

## Inputs stability

The action inputs/outputs in `action.yml` are intended to remain stable across patch releases.
If an input/output needs to change incompatibly, the change should be documented and released with an appropriate version bump.

## Input validation and tests

The composite action validates its inputs before installing or running the CLI:

- `path` must be a relative path inside the checked-out repository and must exist.
- `format` must be one of `text`, `json`, or `sarif`.
- `min-severity` must be one of `critical`, `high`, `medium`, `low`, or `info`.
- `upload-sarif` accepts boolean-like values and is normalized to `true` or `false`.
- `sarif-output` must be a relative output path and cannot include path traversal segments.

Action helper logic lives in `scripts/action_inputs.py`, `scripts/action_summary.py`, and `scripts/resolve_action_version.py`.
Unit fixtures live under `tests/action/fixtures/`, and CI runs them with:

```bash
python -m unittest discover -s tests/action -p "test_*.py"
```
