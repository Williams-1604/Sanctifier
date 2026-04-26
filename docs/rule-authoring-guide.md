# Rule Authoring Guide

> A comprehensive guide for contributors who want to write, test, and ship custom
> analysis rules for Sanctifier.

---

## Table of Contents

1. [Introduction](#introduction)
2. [Rule Anatomy](#rule-anatomy)
3. [Built-in Rules](#built-in-rules)
4. [Authoring a Custom Rule](#authoring-a-custom-rule)
5. [Rule Configuration in `.sanctify.toml`](#rule-configuration-in-sanctifytoml)
6. [YAML Rule Files](#yaml-rule-files)
7. [Testing Your Rule](#testing-your-rule)
8. [Severity Guidelines](#severity-guidelines)
9. [Output Stability and SARIF](#output-stability-and-sarif)
10. [Contribution Notes](#contribution-notes)
11. [Common Pitfalls](#common-pitfalls)
12. [Reference](#reference)

---

## Introduction

Sanctifier ships with a set of **built-in rules** (finding codes `S001`–`S012`)
that cover the most common smart-contract issues on Soroban. In addition, the tool
supports **custom rules** — user-defined patterns that can be added inline in
`.sanctify.toml` or via a standalone YAML file.

This guide explains:

- How rules are structured and executed.
- How to write, test, and document a new rule.
- How to submit a rule contribution through a pull request.

---

## Rule Anatomy

Every Sanctifier rule, whether built-in or custom, shares the same logical
structure:

```
┌──────────────────────────────────────┐
│  1. Matcher   — WHAT to look for     │
│  2. Severity  — HOW bad is it?       │
│  3. Message   — WHAT to tell the dev │
│  4. Metadata  — WHERE to learn more  │
└──────────────────────────────────────┘
```

### Matcher Types

| Type               | Description                                     | Example Use Case                        |
| ------------------ | ----------------------------------------------- | --------------------------------------- |
| `function_call`    | Matches calls to a named function               | Detect `unsafe_transfer` usage          |
| `storage_operation`| Matches `set`/`get`/`remove` on storage keys    | Ensure admin key writes emit events     |
| `method_call`      | Matches method calls on a specific receiver type| Flag `.transfer()` without balance check|
| `regex`            | Arbitrary regex over source text                 | Catch `panic!()` or hardcoded addresses |

### Severity Levels

| Level     | When to Use                                         |
| --------- | --------------------------------------------------- |
| `error`   | Exploitable vulnerability or definite bug            |
| `warning` | Risky pattern that *might* be intentional            |
| `info`    | Informational observation, no immediate risk         |

---

## Built-in Rules

The following rules are compiled into `sanctifier-core` and enabled/disabled via
the `enabled_rules` field in `.sanctify.toml`. See
[error-codes.md](error-codes.md) for full details.

| Code   | Key            | Category              |
| ------ | -------------- | --------------------- |
| `S001` | `auth_gaps`    | Missing `require_auth` |
| `S002` | `panics`       | `panic!`/`unwrap`/`expect` usage |
| `S003` | `arithmetic`   | Unchecked overflow/underflow |
| `S004` | `ledger_size`  | Ledger entry size limits |
| `S005` | —              | Storage-key collisions |
| `S006` | —              | Unsafe patterns |
| `S007` | —              | Custom rule match |
| `S008` | —              | Event topic issues |
| `S009` | —              | Unhandled `Result` |
| `S010` | —              | Upgrade/admin risks |
| `S011` | —              | Z3 invariant violation |
| `S012` | —              | SEP-41 token interface deviations |

---

## Authoring a Custom Rule

### Option 1 — Inline in `.sanctify.toml`

The fastest way to add a rule. Inline rules use **regex matching** against the
source text.

```toml
# .sanctify.toml

[[custom_rules]]
name = "no_unsafe_block"
pattern = "unsafe\\s*\\{"
severity = "error"

[[custom_rules]]
name = "no_mem_forget"
pattern = "std::mem::forget"
severity = "warning"
```

**Required fields:**

| Field      | Type   | Description                                    |
| ---------- | ------ | ---------------------------------------------- |
| `name`     | string | Unique, snake_case identifier                  |
| `pattern`  | string | Regex pattern (escaped for TOML)               |
| `severity` | string | One of `error`, `warning`, `info`              |

### Option 2 — YAML Rule File

For richer rules with structured matchers, use a YAML file. Reference it from
`.sanctify.toml`:

```toml
custom_rules_yaml = "custom-rules.yaml"
```

Then define the rules:

```yaml
# custom-rules.yaml

- id: no_unsafe_transfer
  name: No Unsafe Transfer
  description: Avoid using unsafe_transfer function
  severity: error
  matcher:
    type: function_call
    name: unsafe_transfer
    args: []

- id: require_admin_event
  name: Require Admin Event
  description: Admin changes must emit events
  severity: warning
  matcher:
    type: storage_operation
    operation: set
    key_pattern: "*admin*"
```

**Required fields for YAML rules:**

| Field         | Type   | Description                                    |
| ------------- | ------ | ---------------------------------------------- |
| `id`          | string | Unique, kebab-case identifier                  |
| `name`        | string | Human-readable rule name                       |
| `description` | string | One-sentence summary                           |
| `severity`    | string | One of `error`, `warning`, `info`              |
| `matcher`     | object | Matcher definition (see [Matcher Types](#matcher-types)) |

---

## Rule Configuration in `.sanctify.toml`

The project-level config file controls which rules run:

```toml
# Paths to ignore during analysis
ignore_paths = ["target", ".git"]

# Enable specific built-in rule keys
enabled_rules = ["auth_gaps", "panics", "arithmetic", "ledger_size"]

# Ledger size threshold for S004
ledger_limit = 64000

# Enable strict mode (all built-in rules, no severity downgrade)
strict_mode = false

# Optional: path to YAML custom rules
# custom_rules_yaml = "custom-rules.yaml"

# Inline regex rules
[[custom_rules]]
name = "no_unsafe_block"
pattern = "unsafe\\s*\\{"
severity = "error"
```

### Behavior Notes

- **`enabled_rules`** — Only the listed keys are active. Omitting this field
  enables *all* built-in rules.
- **`strict_mode = true`** — Forces all built-in rules on and prevents custom
  severity overrides. Recommended for CI.
- **`ignore_paths`** — Glob patterns relative to the project root. The `target/`
  directory is always ignored.
- **Custom rules** always run regardless of `enabled_rules`; they are additive.

---

## YAML Rule Files

See [`custom-rules.example.yaml`](../custom-rules.example.yaml) in the project
root for a complete working example.

### Matcher Reference

#### `function_call`

```yaml
matcher:
  type: function_call
  name: unsafe_transfer    # exact function name
  args: []                 # (reserved for future arg-pattern matching)
```

#### `storage_operation`

```yaml
matcher:
  type: storage_operation
  operation: set           # set | get | remove
  key_pattern: "*admin*"   # glob pattern over the storage key
```

#### `method_call`

```yaml
matcher:
  type: method_call
  method: transfer         # method name
  receiver: "*Client"      # glob pattern for the receiver type
```

#### `regex`

```yaml
matcher:
  type: regex
  pattern: "panic!\\("     # regex over source text (double-escape in YAML)
```

---

## Testing Your Rule

### Writing Test Fixtures

For every rule, create a fixture directory under
`contracts/fixtures/finding-codes/` with positive and negative examples:

```
contracts/fixtures/finding-codes/
├── s007_custom_no_unsafe_block/
│   ├── should_match.rs       # Code that SHOULD trigger the rule
│   └── should_not_match.rs   # Code that SHOULD NOT trigger the rule
```

**`should_match.rs`** — demonstrates the pattern:

```rust
// This should trigger `no_unsafe_block`
pub fn dangerous() {
    unsafe {
        // ...
    }
}
```

**`should_not_match.rs`** — demonstrates clean code:

```rust
// This should NOT trigger `no_unsafe_block`
pub fn safe_fn() {
    let result: Result<(), &str> = Ok(());
    result.unwrap_or_default();
}
```

### Running Tests Locally

```bash
# Run the full test suite
cargo test -p sanctifier-core --all-features

# Run only rule-related tests
cargo test -p sanctifier-core -- rules

# Lint check
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

### CI Validation

All PRs are validated by the `Continuous Integration` workflow, which runs:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`

Ensure your rule passes all three before submitting.

---

## Severity Guidelines

Choose severity based on **exploitability** and **blast radius**:

| Severity  | Criteria                                            | Example                                |
| --------- | --------------------------------------------------- | -------------------------------------- |
| `error`   | Directly exploitable; funds at risk                 | Missing `require_auth` on withdrawal   |
| `warning` | Risky but may be intentional or context-dependent   | `unwrap()` in a non-critical path      |
| `info`    | Observation only; no security impact                | Function naming convention violation   |

> **Tip:** When in doubt, start with `warning`. Reviewers will suggest adjusting
> severity during PR review.

---

## Output Stability and SARIF

Sanctifier produces findings in a **stable JSON format** that is compatible with
SARIF (Static Analysis Results Interchange Format). When authoring rules:

- **Do not change** the shape of the `findings` array or the `code` field format.
- Custom rules are reported with code `S007` in the output.
- If your change alters the output schema, you **must** include:
  - A version bump in the output format.
  - Migration notes in the PR description.

See [SARIF_METADATA.md](SARIF_METADATA.md) for the current output schema.

---

## Contribution Notes

### Before You Start

1. **Open an issue first** if you're proposing a new built-in rule. Custom rule
   contributions (YAML/TOML examples) can go straight to PR.
2. **Check existing rules** — your idea may overlap with `S001`–`S012` or an
   existing custom rule example.
3. **Read [CONTRIBUTING.md](../CONTRIBUTING.md)** for general PR process, commit
   conventions, and branch protection requirements.

### PR Checklist for Rule Contributions

- [ ] Rule has a unique, descriptive identifier
- [ ] Severity is appropriate and justified in the PR description
- [ ] At least 1 positive and 1 negative test fixture provided
- [ ] `cargo test` passes locally
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] Rule is documented (description, rationale, example match/no-match)
- [ ] If modifying output format: version bump + migration notes included

### Branch Naming

Follow the convention from [CONTRIBUTING.md](../CONTRIBUTING.md):

| Type                     | Branch pattern                    |
| ------------------------ | --------------------------------- |
| New built-in rule        | `rule/<rule-name>`                |
| New custom rule example  | `docs/custom-rule-<name>`         |
| Fix to existing rule     | `fix/rule-<rule-id>`              |

### Commit Message

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(rules): add no-direct-panic custom rule example
fix(rules): correct false positive in auth_gaps for admin-only paths
docs: update rule authoring guide with YAML matcher reference
```

---

## Common Pitfalls

| Pitfall                          | Solution                                            |
| -------------------------------- | --------------------------------------------------- |
| Regex too broad → false positives| Use anchors, word boundaries, or structured matchers |
| Regex too narrow → false negatives| Test with variations (whitespace, comments, macros)  |
| Missing TOML escaping            | Double-escape backslashes: `\\s` not `\s`           |
| Severity too high                | Start with `warning`; escalate during review         |
| No test fixtures                 | Always provide `should_match` + `should_not_match`   |
| Breaking output format           | Version bump + migration notes required              |

---

## Reference

| Resource                                                                   | Description                          |
| -------------------------------------------------------------------------- | ------------------------------------ |
| [Error Codes](error-codes.md)                                              | Full `S001`–`S012` code table        |
| [Contributing Analysis Rules](Contributing-analysis-rules.MD)              | Static analysis + formal verification guide |
| [Custom Rules Example](../custom-rules.example.yaml)                      | Working YAML rule file               |
| [Vulnerability Database Format](vulnerability-database-format.md)          | `SOL-2024-*` entry format            |
| [CONTRIBUTING.md](../CONTRIBUTING.md)                                      | General contribution guidelines      |
| [`.sanctify.toml`](../.sanctify.toml)                                      | Project config reference             |

---

*Last updated: April 2026 | License: see project root LICENSE file*
