# SCTR security model

## Security objective

A tenant may control only registered applications owned by that tenant. A
lifecycle request must not allow a tenant to control another tenant, alter a
root-owned unit, execute as root, access another tenant's logs, or invoke an
arbitrary systemd operation.

Rust contributes memory safety. The actual authorization boundary is the SCTR
policy, the Linux identity model, root-owned provisioning, and systemd's unit
execution rules.

## Assets

- tenant application availability and integrity;
- root-owned unit files and the application registry;
- tenant release directories and deployment state;
- systemd D-Bus access;
- logs and deployment audit records;
- host and service-account privileges.

## Trust boundaries

```text
browser or deploy client
        ↓ authenticated request
SCTR API / local client boundary
        ↓ tenant and application policy
SCTR policy and registry
        ↓ exact typed systemd operation
systemd system manager
        ↓ User=tenant and cgroup policy
tenant application
```

The systemd manager is privileged. A public HTTP process must not expose it
through a generic proxy or run as root. If a privileged helper is required, it
must have no public listener, accept typed requests only, and revalidate the
exact unit against the root-owned registry.

## Controls

### Identity

Tenant workloads use existing Linux accounts from the host account database. For
example, `projenv` and `marcuscosta` are tenant runtime identities when the
registry maps applications to them. SCTR must not create a parallel
`sctr-tenant-<id>` account for an existing tenant. The `sctr-agent` identity is
separate: it belongs to the control plane and never runs tenant workloads.

A local client may use the kernel-provided peer UID on a protected Unix socket.
A web request must use the authenticated hosting identity from a trusted
session or service-to-service credential. A request field named `user` or
tenant is not an identity proof.

### Authorization

Authorization is an exact lookup:

```text
authenticated tenant + application ID + action
    → registry entry
    → exact unit name and allowed action
```

Unknown and unauthorized applications should not disclose whether another
tenant's application exists. Do not accept a raw unit name, PID, path, command,
UID, or arbitrary D-Bus method from the client.

### Unit and path safety

- Unit files and the registry are root-owned and not tenant-writable.
- Tenant runtime UIDs are resolved from the operating-system account database.
- UID 0 and other explicitly privileged identities are rejected for tenant apps.
- Application roots are canonicalized and constrained to the tenant root.
- The provisioner verifies traversal and ownership permissions for the existing
  tenant root under `/opt/home`; it does not weaken the shared parent blindly.
- Unit names are generated from validated identifiers, not concatenated from
  untrusted strings.
- The privileged provisioner never writes to a tenant-controlled path based on
  an unvalidated request.
- Release selection is atomic and remains inside the registered application
  root.

### D-Bus safety

The Rust systemd adapter exposes concrete lifecycle operations instead of a
generic D-Bus call function. It uses safe job modes, bounded deadlines, and
waits for the matching job result. It does not expose unit installation,
transient units, arbitrary property changes, global reload, shutdown, or
system reset to tenants.

### Runtime safety

Every unit executes with the tenant UID/GID and receives a cgroup policy. The
first policy should consider `MemoryMax`, `CPUQuota`, `TasksMax`, `LimitNOFILE`,
restart limits, and a controlled port allocation. Sandboxing options such as
`NoNewPrivileges`, `PrivateTmp`, and filesystem restrictions are enabled only
when compatible with the application contract.

### Logs and audit

Logs are selected by registered unit, not by arbitrary filesystem path. SCTR
must bound log reads and redact credentials or tokens. Audit events record the
actor, tenant, application, action, unit, result, and correlation ID without
copying secrets or full command lines.

## Abuse cases

| Abuse case | Required control |
|---|---|
| Alice requests Bob's app | Registry ownership check before D-Bus |
| Client submits a root unit | Ignore client unit; derive exact unit internally |
| Client submits `../` path | Canonicalize and reject outside registered root |
| App starts as root | Reject registry/provisioning configuration |
| D-Bus job is queued but fails | Await the matching job result and report failure |
| App writes runaway resources | Apply systemd cgroup limits |
| User requests arbitrary logs | Resolve logs by unit and tenant policy |
| Public API is compromised | Keep privileged helper local and minimize its method set |
| Malicious release changes command | Keep command in root-owned unit; release changes code only |

## Required proof

Security is not complete until tests demonstrate:

- cross-tenant lifecycle requests are denied;
- a tenant cannot change another tenant's unit or release path;
- root-owned units cannot be rewritten by tenant accounts;
- processes report the expected UID/GID;
- arbitrary unit names, paths, commands, and D-Bus methods are rejected;
- logs do not cross tenant boundaries;
- failed, timed-out, and cancelled systemd jobs are surfaced safely.
