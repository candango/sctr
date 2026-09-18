# Project Guidance

SCTR is a Linux hosting control plane for non-PHP application deployments.
The target implementation is Rust. The runtime is systemd. Nginx and
per-account PHP-FPM remain outside SCTR.

## Product direction

SCTR provides a controlled lifecycle for tenant applications:

- register an application;
- provision a root-owned systemd unit;
- start, stop, restart, and inspect the application;
- expose unit-scoped status and logs;
- support deployment release switching and rollback.

Each application gets one system service unit. The unit must execute the
application with the tenant's UID and primary group. PHP and HTML are served by
Nginx and PHP-FPM and are not managed by SCTR.

Supervisor is legacy context only. Do not add Supervisor, `supervisorctl`,
Pebble, or a Python compatibility layer to the Rust implementation unless a
new accepted design decision changes this boundary.

## Security boundary

SCTR is a policy layer over systemd, not a generic systemd proxy.

- Clients identify an application, never a unit path, UID, command, or socket.
- The registry maps an authenticated tenant and application ID to one exact
  unit name and one allowed action set.
- Unit files and the registry are root-owned and are not writable by tenants.
- Tenant processes run with `User=` and `Group=` for the tenant; root execution
  is rejected for tenant applications.
- SCTR must not expose unrestricted `systemctl` or generic D-Bus methods.
- The D-Bus adapter exposes a small typed operation set and waits for job
  completion before reporting success.
- Public HTTP handling must not run as root. Any privileged helper must have a
  narrow local IPC boundary and independently validate the requested unit.
- Commands are argv-based and absolute; do not invoke a shell for lifecycle
  control.
- Paths are canonicalized and constrained to the tenant's registered root.
- Logs are scoped to the registered unit and must not disclose other tenants'
  data.

## Rust policy

Use Rust 2024 or the version selected by the first Cargo manifest, with an
explicit supported Rust version. Prefer a single package until a real crate or
binary boundary exists; use a workspace when multiple real boundaries earn it.

Use `Result` and typed errors. `thiserror` is the accepted default for domain
and boundary error enums. An application-level error aggregator may be added
only when it improves the CLI or service boundary and is documented first.
Do not use `unwrap` or `expect` in runtime paths without a documented invariant.
Keep unsafe code out of the initial implementation.

The initial conventional checks are:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Run dependency auditing when the dependency surface is introduced or changed.
Do not add a framework or crate merely to satisfy an architectural pattern.

## Documentation contract

Durable project decisions belong in `docs/` and must be written in English.
Keep these concerns separate:

- `rust-systemd-design.md`: architecture and behavior contract;
- `security.md`: threats, trust boundaries, and controls;
- `testing.md`: unit, integration, and end-to-end proof;
- `deployment.md`: provisioning, release, rollback, and operations.

The old Python/Go audit is historical and must be marked superseded rather than
used as the current implementation contract.

## Task and implementation workflow

This initiative uses `[EXECUTE]` tasks. Work remains scoped to the active
Taskwarrior task, but implementation is direct and every material change is
explained to the learner in chat.

Break work into small initiatives and tasks. Every task needs a concrete
outcome and verification evidence. Record decisions, research, lessons, and
outcomes in Taskwarrior before closing the task. Map each initiative to a
GitHub issue through the approved broker; never use raw GitHub CLI commands.

The first usable target is version `0.0.1`, but the version is not a promise to
release until the end-to-end acceptance criteria are met.
