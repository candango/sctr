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

- **Tenant:** a hosting account mapped to one existing operating-system UID/GID.
- **Tenant account:** the existing Linux account for that hosting tenant, such as `projenv` or `marcuscosta`; SCTR does not create a parallel `sctr-tenant-<id>` account for it.
- **SCTR agent:** a separate control-plane service identity, when deployed; it never runs tenant workloads.
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
  └── reverse proxy ── HTTP/WebSocket ────> sctr-projenv-api.service
                                             User=projenv

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
Description=SCTR application projenv/api
After=network-online.target

[Service]
Type=exec
User=projenv
Group=projenv
WorkingDirectory=/opt/home/projenv/apps/api/current
ExecStart=/opt/home/projenv/apps/api/current/bin/api
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

The provisioner must resolve the actual primary group of the existing tenant
account and reject privileged runtime identities for tenant applications. For
example, an application owned by `projenv` runs with `User=projenv` and its
resolved primary `Group=`; an application owned by `marcuscosta` uses that
existing account instead. Commands are absolute and are not wrapped in a shell.

The POC's `projenv-poc-*` users were disposable test identities only. They are
not a production naming scheme and must not replace the existing accounts under
`/opt/home`. Environment, port, resource, and sandbox settings come
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

1. Resolve and validate the existing tenant operating-system account (for
   example, `projenv` or `marcuscosta`) and application identifier; do not
   create a parallel `sctr-tenant-<id>` account.
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

## Consultancy handoff for SCTR

This section is an integration brief for the separate SCTR project. It is not an implementation plan for this provisioning repository and does not authorize Rust code changes here.

### Validated boundary

SCTR owns customer tenant workloads: application services, workers, consumers,
and tenant jobs or timers. Platform daemons remain outside SCTR and continue
to be managed by the provisioning and systemd configuration layers.

The target runtime is the system-wide systemd manager. SCTR must not depend on
`systemd --user`. The registry remains the authority that maps an authenticated
tenant and application identifier to one exact unit name, runtime identity,
allowed actions, and resource policy.

### Minimal typed control contract

The public SCTR API should accept an application identifier and a typed action,
never a unit path, UID, command, socket, or arbitrary D-Bus method. The initial
operation set is:

- start one registered application;
- stop one registered application;
- restart one registered application;
- read status for one registered application;
- read bounded logs for one registered application.

The privileged systemd adapter should resolve the exact unit from the registry,
call only the required Manager methods, and await the resulting job before
returning success:

- `StartUnit` for start;
- `StopUnit` for stop;
- `RestartUnit` for restart;
- `GetUnit` and a bounded property set for status;
- the unit-scoped journal adapter for logs.

`ListUnits` may support administrative reconciliation, but it must not become
a tenant-facing discovery or generic proxy operation. The adapter must reject
unknown applications, cross-tenant mappings, privileged runtime identities,
unit mismatches, unsupported actions, and expired or failed jobs with typed
errors. D-Bus calls need bounded timeouts and must report the systemd job result,
not merely successful request submission.

### Unit materialization recommendation

For the `0.0.1` product, prefer persistent root-owned unit files generated during
provisioning. This aligns with boot recovery, operator inspection, registry/unit
drift detection, and rebuilding a missing unit from the registry. Runtime
lifecycle operations should control those exact units through the typed adapter;
they should not rewrite unit files or call `daemon-reload` during an ordinary
release deployment.

Transient units through `StartTransientUnit` can remain a future option for
explicitly ephemeral jobs. They should not replace the persistent application
unit model until SCTR has an explicit contract for boot persistence, recovery,
inspection, and ownership of transient unit properties.

### Validation evidence and sizing boundary

A disposable two-tenant validation passed on Rocky Linux 10.2 with SELinux
Enforcing. It confirmed separate runtime identities, systemd Manager D-Bus
lifecycle operations, `Restart=on-failure`, `KillMode=control-group`,
`MemoryMax=64M`, `CPUQuota=50%`, `TasksMax=16`, `NoNewPrivileges`,
`ProtectSystem=strict`, `ProtectHome=yes`, protected unit files, child-process
cleanup, tenant isolation, and no recent matching AVC denials. All temporary
users, units, and directories were removed after the test.

The POC values are validation fixtures, not production defaults. CPU, memory,
and task/thread budgets require workload measurements and safety margins; that
sizing work is tracked separately in GitHub issue `projenv/provisioning#29`.

## Application provisioning and lifecycle flow

The create-and-run path is a privileged SCTR operation. A tenant or client
identifies an application; it does not create a systemd unit, provide a unit
path, or submit arbitrary execution parameters.

```text
admin/deployer or SCTR API client
  → SCTR provision(application)
  → validate tenant, application ID, root, command, identity, and policy
  → allocate the exact internal unit name
  → write the root-owned registry entry and unit source
  → install the unit and run daemon-reload
  → call StartUnit for the registered application
  → await the systemd job result and query service status
  → return running or a structured failure
```

Provisioning is administrative and may create or rebuild the persistent unit.
`daemon-reload` belongs to this path, not to an ordinary release deployment.
The unit must be root-owned, execute with the registered tenant `User=` and
`Group=`, and contain only validated absolute commands and policy-controlled
resource and sandbox settings.

After provisioning, the public lifecycle contract is intentionally small:

```text
start(application_id)
stop(application_id)
restart(application_id)
status(application_id)
logs(application_id, bounded_query)
```

Each operation resolves `application_id` through the registry, verifies the
caller and allowed action set, maps to one exact unit, and reports the systemd
job result plus the resulting service state. A request is not successful merely
because systemd accepted a job.

The control path must return structured failures for unknown applications,
unauthorized actions, registry conflicts, invalid deployment state, unit
installation failures, systemd job failures, timeouts, and status mismatches.
It must never fall back to arbitrary `systemctl` execution or a generic D-Bus
proxy.
