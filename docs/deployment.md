# SCTR deployment and operations

## Hosting layout

```text
/etc/sctr/
├── registry.json                 # root-owned policy state
└── units/                        # generated unit sources

/etc/systemd/system/
└── sctr-<tenant>-<app>.service   # root-owned installed units

/home/<tenant>/apps/<app>/
├── releases/<release-id>/        # tenant-owned immutable releases
└── current -> releases/<id>       # controlled active release
```

The exact filesystem layout may change during implementation, but the
ownership rule does not: registry and unit files are privileged state; release
contents belong to the tenant and execute as the tenant.

## Provision an application

Provisioning is an administrative operation:

1. create or validate the tenant operating-system account;
2. allocate the application identifier and internal unit name;
3. create the tenant application root and release layout;
4. validate the runtime and absolute command;
5. generate the root-owned systemd unit with `User=` and `Group=`;
6. install the unit and reload the systemd manager;
7. register the application policy and allowed actions;
8. start only when the application policy requests it.

The client cannot create arbitrary units or change `User=`, `ExecStart=`,
`WorkingDirectory=`, limits, or privileged systemd properties.

## Deploy a release

A release operation is intentionally narrower than provisioning:

```text
upload release
  → validate expected files
  → write release outside current
  → atomically switch current
  → RestartUnit(exact application unit)
  → await job result
  → query status and health
```

If restart or health validation fails, the deployment keeps the previous
release available and may switch back through the rollback operation. SCTR must
not report success merely because systemd accepted a job.

## Lifecycle operations

The first public operations are:

```text
list, status, start, stop, restart, logs
```

`restart` is the default deployment lifecycle action. `reload` is supported
only for applications whose unit explicitly defines a safe `ExecReload=`
contract. `daemon-reload` is an administrative operation for changed unit files
and is never a tenant deployment action.

## Logs and status

SCTR reads status from systemd unit and service properties. Logs are read from
the journal by exact unit identity and bounded by time and size. Do not expose a
generic journal query or arbitrary path reader to a tenant.

## Recovery

Operators must be able to:

- identify a failed unit and its last safe release;
- stop a runaway application through the privileged control path;
- restore the previous release;
- rebuild a missing unit from the registry;
- detect registry/unit drift;
- disable an account without touching other tenants.

Recovery actions are audited and require the same exact tenant/application
mapping as ordinary lifecycle actions, with explicit administrative authority
for cross-tenant operations.
