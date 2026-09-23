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
- the existing tenant account can traverse `/opt/home` and its registered root
  without broadening shared parent permissions;
- unit names cannot escape the SCTR namespace;
- only an approved provisioning path can trigger manager reload.

## End-to-end matrix

The first end-to-end environment should use two disposable, non-root lab
identities for the POC and at least these applications. Production workloads
must use existing tenant accounts such as `projenv` and `marcuscosta`; the
POC identities must not become a production account naming scheme.

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

## Rocky lab systemd POC

This section records the disposable systemd validation performed for the SCTR
integration boundary and gives an authorized operator a safe way to inspect the
Rocky lab host.

This is lab evidence, not a production-readiness claim. The POC creates
temporary users, directories, and root-owned units, exercises them, and removes
them before it exits.

### Validated target

- Host: `rocky10` / `rocky10-01`
- Address: `192.168.0.206`
- Operating system: Rocky Linux 10.2
- systemd: 257
- SELinux: Enforcing
- Runtime model: system-wide systemd manager, not `systemd --user`
- Tenant access: separate non-root `User=` and `Group=` identities

The POC used disposable users only to prove isolation. Production units must
run as the existing tenant accounts resolved from the host account database,
such as `projenv` or `marcuscosta`; `sctr-tenant-<id>` is not a production
account naming scheme. A separate `sctr-agent` control-plane account, when
used, must never run tenant workloads.

The local provisioning inventory is the source of truth for the lab host and
operator account. It references an authorized private key stored outside Git.
Never copy the private key, password, vault contents, or credential output into
this repository.

The lab now grants `projenv` an explicit execute-only ACL on `/opt/home`;
`marcuscosta` remains the parent owner. Read-only validation confirms both
`projenv` and `marcuscosta` can traverse their registered roots. The ACL grants
traversal without directory listing or read access; production provisioning
must apply the same approved policy idempotently and must not weaken the parent
permissions blindly.

### Safe access for an authorized operator

Set these values from the approved local inventory or credential store. Do not
replace the key with a key pasted into a ticket or document:

```bash
export SCTR_LAB_HOST=192.168.0.206
export SCTR_LAB_USER=usradm
export SCTR_LAB_SSH_KEY=/home/fpiraz/keys/local/id_rsa_usradm
```

Use `IdentitiesOnly=yes` so unrelated SSH-agent keys are not offered to the
host:

```bash
ssh \
  -o BatchMode=yes \
  -o IdentitiesOnly=yes \
  -o ConnectTimeout=5 \
  -i "$SCTR_LAB_SSH_KEY" \
  "$SCTR_LAB_USER@$SCTR_LAB_HOST"
```

The expected host preflight is read-only:

```bash
ssh -o BatchMode=yes -o IdentitiesOnly=yes \
  -i "$SCTR_LAB_SSH_KEY" \
  "$SCTR_LAB_USER@$SCTR_LAB_HOST" \
  'hostname; id; systemctl --version | head -1; getenforce; sudo -n true'
```

A successful preflight should show `rocky10-01`, the authorized operator, a
systemd 257 version, `Enforcing`, and passwordless non-interactive sudo for the
lab operator account. This preflight was verified with the approved local key;
it returned `rocky10-01`, `usradm`, systemd 257, and `Enforcing`.

If SSH reports `Too many authentication failures`, keep `IdentitiesOnly=yes`
and confirm that the approved key is the one selected by the local inventory.

### Manager and D-Bus inspection

The following checks do not create or modify services:

```bash
ssh -o BatchMode=yes -o IdentitiesOnly=yes \
  -i "$SCTR_LAB_SSH_KEY" \
  "$SCTR_LAB_USER@$SCTR_LAB_HOST" \
  'sudo busctl introspect org.freedesktop.systemd1 \
     /org/freedesktop/systemd1 \
     org.freedesktop.systemd1.Manager --no-pager \
   | grep -E "(ListUnits|GetUnit|StartUnit|StopUnit|RestartUnit)"'
```

The system manager must expose the lifecycle methods used by the typed SCTR
adapter. SCTR must still resolve an application ID through its registry and
must not expose this introspection or a generic D-Bus proxy to tenants.

For an existing registered application, replace `<unit>` with the unit resolved
by the authorized SCTR operator. Do not accept a unit name from an untrusted
client:

```bash
ssh -o BatchMode=yes -o IdentitiesOnly=yes \
  -i "$SCTR_LAB_SSH_KEY" \
  "$SCTR_LAB_USER@$SCTR_LAB_HOST" \
  'sudo systemctl show <unit> \
     -p User -p Group -p MainPID -p ActiveState \
     -p Restart -p KillMode -p MemoryMax \
     -p CPUQuotaPerSecUSec -p TasksMax \
     -p NoNewPrivileges -p ProtectSystem -p ProtectHome'
```

Useful read-only observation commands are:

```bash
sudo systemd-cgtop
sudo systemd-cgls
sudo journalctl -u <unit> --since '-10 minutes' --no-pager
```

Logs must remain scoped to the exact registered unit and bounded by time and
size. Do not use a generic journal query for tenant-facing access.

### POC acceptance checks

A disposable two-tenant run is successful when all of the following pass:

- `systemd-analyze verify` accepts both generated units;
- both units start under separate non-root tenant identities;
- `StartUnit`, `StopUnit`, `RestartUnit`, `GetUnit`, and `ListUnits` work for
  the system manager through the authorized control path;
- `MemoryMax=64M`, `CPUQuota=50%`, and `TasksMax=16` are applied;
- `Restart=on-failure` recovers a unit after its main process is killed;
- `KillMode=control-group` removes child processes on stop;
- tenant A cannot stop tenant B or read tenant B's root-owned unit file;
- `NoNewPrivileges=yes`, `ProtectSystem=strict`, and `ProtectHome=yes` are
  applied;
- SELinux remains enforcing with no matching AVC denials;
- cleanup removes temporary users, units, directories, and failure state.

The validated fixture values are not production defaults. In particular,
`CPUQuota=50%` is half of one CPU core, and `TasksMax=16` counts processes and
threads in the unit cgroup. Workload sizing is tracked separately in
`projenv/provisioning#29`.

### Cleanup and safety

The POC is disposable. Any rerun must:

1. use unique names for temporary users, directories, and units;
2. use root-owned unit files with restrictive permissions;
3. stop units before removing their files;
4. run `systemctl daemon-reload` after removing unit files;
5. remove temporary users and directories;
6. verify no temporary unit or process remains;
7. inspect recent SELinux AVC records for the unique POC prefix.

Do not run the POC against a production host, the historical `192.168.0.200`
host, or `rocky10-2` without explicit target authorization. Do not place
credentials in command output, logs, screenshots, issue bodies, or this
repository.
