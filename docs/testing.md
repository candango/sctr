# SCTR testing and validation

## Quality gates

The initial Rust baseline is:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Run dependency auditing when the dependency graph is introduced or changed.
The repository must also build from a clean checkout before a release is
considered.

## CLI contract tests

The bootstrap command surface is tested without a live systemd manager. Tests
must prove that empty invocation, `help`, and `--help` share one custom
renderer; nested help resolves through the same command tree; unknown syntax
returns an actionable usage error; and unimplemented lifecycle operations fail
closed without side effects.

## Unit tests

Unit tests must cover the domain without requiring a live systemd manager:

- tenant and application identifier validation;
- registry lookup and ownership policy;
- allowed and denied action matrices;
- unit-name generation;
- path containment and release selection;
- resource policy validation;
- safe public error mapping;
- audit record redaction.

Use table-driven cases for the policy matrix. Keep the policy independent from
D-Bus so the security contract can be tested deterministically.

## Systemd adapter tests

The adapter must be tested at its D-Bus boundary. The test seam should verify:

- exact unit name and method selected;
- safe job mode and deadline;
- matching job completion is awaited;
- successful and failed job results;
- missing units and inactive units;
- reload unsupported by a unit;
- cancellation and timeout handling;
- malformed or unexpected property values.

Do not replace the entire boundary with a mock that never checks the actual
method and arguments. Add a Linux integration path against a disposable
systemd service when the environment supports it.

## Provisioning tests

Provisioning tests must prove:

- generated units are root-owned and not tenant-writable;
- `User=` and `Group=` match the registered account;
- privileged runtime identities are rejected;
- commands are absolute and not shell-wrapped;
- application and release paths stay within the registered root;
- unit names cannot escape the SCTR namespace;
- only an approved provisioning path can trigger manager reload.

## End-to-end matrix

The first end-to-end environment should create two unprivileged tenants and
at least these applications:

- a long-running Go or shell-free test binary;
- an application that exits immediately;
- an application that becomes unhealthy;
- an initially stopped application.

Prove all of the following:

1. Alice can list and control only Alice's applications.
2. Bob cannot start, stop, restart, inspect, or read Alice's application.
3. A process runs with the configured tenant UID/GID.
4. A release switch followed by restart uses the new release.
5. A failed start returns a failed deployment result.
6. Stop terminates the complete service cgroup.
7. Resource limits are applied to the unit.
8. Logs are scoped to the requested application.
9. Rollback restores the previous release.
10. Systemd or D-Bus failures do not become false successes.

## Release acceptance

`0.0.1` is acceptable only when the smallest end-to-end path is proven:

```text
register tenant/app
  → generate unit
  → start app
  → query status
  → read scoped logs
  → stop/restart app
  → deploy a release
  → verify rollback
```

A green unit test suite without this matrix is not sufficient evidence for a
multi-tenant hosting control plane.
