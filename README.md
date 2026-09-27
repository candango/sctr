# SCTR

## Introduction

SCTR is a Rust/systemd hosting control plane for long-running non-PHP
applications. It provisions and controls one systemd unit per application,
runs tenant applications with the tenant's operating-system identity, and
exposes scoped lifecycle, status, and log operations.

Nginx and per-account PHP-FPM remain responsible for PHP and HTML. SCTR is for
Node.js, Python, Go, Rust, workers, WebSockets, schedulers, and similar
long-running applications.

The first target is a small, auditable control plane rather than a generic
systemd proxy. The `0.0.1` contract will be accepted only after the smallest
end-to-end hosting workflow is proven.

## Documentation

- [Rust/systemd design](docs/rust-systemd-design.md)
- [Security model](docs/security.md)
- [Testing and validation](docs/testing.md)
- [Deployment and operations](docs/deployment.md)
- [Command-line contract](docs/cli.md)
- [Legacy Python audit](docs/go-rewrite-audit.md)

## Build

SCTR requires Rust 1.85 or newer and uses the Rust 2024 edition:

```text
cargo build
cargo test
```

The superseded Python/Supervisor implementation is retained only through the
explicitly historical audit document; it is not part of the Rust build or
runtime.

## Usage

The first executable slice provides a registry-backed lifecycle client and
detailed operational help. It reads the root-owned registry at
`/etc/sctr/registry.json` and uses the systemd system manager through typed
D-Bus calls:

```text
sctr help
sctr help app
sctr help app start
```

The accepted application contract is:

```text
sctr app list
sctr app status <app>
sctr app start <app>
sctr app stop <app>
sctr app restart <app>
sctr app logs <app>
```

List, status, start, stop, and restart resolve the application through the
registry, authorize the current actor, call the exact registered unit, await the
systemd job result, and report the resulting state. Logs remain contract-only
until the bounded journal adapter is implemented. Missing or invalid registry
state fails closed with an actionable error.

## Support

The sctr project is one of [Candango Open Source
Group](http://www.candango.org/projects/)
initiatives. It is available under [Apache License,
Version 2.0](http://www.apache.org/licenses/LICENSE-2.0.html).

This website and all documentation are licensed under [Creative
Commons 3.0](http://creativecommons.org/licenses/by/3.0/).
