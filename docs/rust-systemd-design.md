# SCTR Rust/systemd design

**Status:** Accepted foundation for the `0.0.1` initiative.

**Scope:** Linux hosting for non-PHP application deployments.

## Decision

SCTR is implemented in Rust and uses the systemd system manager as its process
runtime. SCTR is the control plane: it owns application identity, tenant
authorization, provisioning, lifecycle requests, and deployment state. systemd
owns process execution, restart policy, cgroups, and service state.

Nginx and per-account PHP-FPM continue to serve PHP and HTML. SCTR does not
replace or control PHP-FPM in the initial product.

## Product boundary

The first application contract is:

```text
app list
app status <app>
app start <app>
app stop <app>
app restart <app>
app logs <app>
```

The client supplies a registered application identifier and an action. It does
not supply a systemd unit name, path, UID, command, socket, or arbitrary D-Bus
method.

A deployment changes a release under the tenant's registered application root,
then asks SCTR to restart the existing unit. SCTR is not a build system and is
not responsible for compiling application code in the first release.

## Terminology

- **Tenant:** a hosting account mapped to one operating-system UID/GID.
- **Application:** a registered long-running process owned by one tenant.
- **Release:** an immutable application directory selected by the tenant's
  controlled `current` link.
- **Unit:** the root-owned systemd service generated for an application.
- **Registry:** root-owned SCTR state mapping tenant and application IDs to
  units, roots, runtime identity, actions, and resource policy.

## Runtime layout

```text
Nginx ──────────────── PHP request ────────> per-account PHP-FPM
  │
  └── reverse proxy ── HTTP/WebSocket ────> sctr-alice-api.service
                                             User=alice

SCTR Rust ── typed D-Bus operations ──────> systemd system manager
```

The generated unit name is stable and derived from a validated internal ID,
for example:

```text
sctr-alice-api.service
sctr-alice-worker.service
```

The name is not accepted directly from an untrusted request. SCTR resolves it
from the registry.

## Unit contract

A generated unit is root-owned and is not writable by the tenant:

```ini
[Unit]
Description=SCTR application alice/api
After=network-online.target

[Service]
Type=exec
User=alice
WorkingDirectory=/home/alice/apps/api/current
ExecStart=/home/alice/apps/api/current/bin/api
Restart=on-failure
RestartSec=5s
KillMode=control-group
TimeoutStopSec=30s
NoNewPrivileges=yes
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

The provisioner must resolve the actual primary group and reject privileged
runtime identities for tenant applications. Commands are absolute and are not
wrapped in a shell. Environment, port, resource, and sandbox settings come
from the validated registry rather than the lifecycle request.

A release deploy normally changes only the `current` selection and restarts
the existing unit. `daemon-reload` is a provisioning operation, not a normal
per-release operation.

## SCTR/systemd boundary

The first typed adapter should provide only the operations the product needs:

- start one registered unit;
- stop one registered unit;
- restart one registered unit;
- reload a unit only when its contract includes a supported reload operation;
- read unit and service properties;
- subscribe to or await the result of the systemd job;
- read logs through a unit-scoped logging adapter.

The adapter must not expose a generic D-Bus method dispatcher. It must use
bounded timeouts, preserve structured errors, and wait for the job result after
`StartUnit`, `StopUnit`, or `RestartUnit` is queued.

`daemon-reload`, unit-file installation, transient unit creation, arbitrary
property mutation, global reset, and system shutdown are administrative
operations. They are not tenant lifecycle actions.

## Provisioning flow

1. Validate the tenant's operating-system account and application identifier.
2. Validate the application root and release layout.
3. Select the approved runtime, command, port, environment references, and
   resource limits.
4. Write the unit and registry entry through the privileged provisioning path.
5. Run a systemd manager reload after unit-file changes.
6. Enable or start the exact unit according to the application policy.
7. Record the unit name, operation, result, and correlation ID in the audit log.

## Deployment flow

```text
upload release
  → validate release layout
  → atomically select current release
  → SCTR RestartUnit(exact registered unit)
  → await job result
  → query status and health
  → retain the previous release for rollback
```

A failed start must not be reported as a successful deployment merely because a
D-Bus request was accepted. The job result and the service state are part of
the operation result.

## Non-goals for 0.0.1

- PHP-FPM management;
- arbitrary shell execution;
- a generic systemd dashboard;
- tenant-created systemd units;
- exposing `systemctl` to tenants;
- distributed orchestration across hosts;
- automatic builds or language-specific package management;
- Supervisor compatibility.

## Future boundaries

A separate crate or binary is justified only when it owns a real boundary, such
as the domain policy library, the systemd adapter, a privileged helper, or a
network API. The first Rust implementation should remain small enough that the
lifecycle path is visible from validation through authorization, D-Bus, audit,
and response.
