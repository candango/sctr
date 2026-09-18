# Legacy SCTR Python audit and Go rewrite brief

> **Status: Superseded.** This document records the legacy Python prototype and
> the earlier Go rewrite direction. It is retained as historical audit context.
> The current implementation contract is documented in
> [Rust/systemd design](rust-systemd-design.md), [security](security.md),
> [testing](testing.md), and [deployment](deployment.md).

**Historical status:** Audit and handoff document. No Go implementation was
included in the audited revision.

**Audited repository:** `candango/sctr`

**Audited revision:** branch `develop`, commit `c116e1d`

**Audit date:** 2026-09-04

**Audience:** The next agent implementing the replacement in Go.

## 1. Executive conclusion

SCTR has a valid product idea but the current implementation is only an early
proof of concept. It is a central Python/Firenado application that authenticates
against a local YAML list and shells out to a shared Supervisor daemon. It
filters running processes by the current OS owner and then invokes
`supervisorctl restart`.

The Go rewrite should preserve the user-facing intent:

- a non-privileged operator controls only their own applications;
- an administrator provisions applications and their runtime identity;
- process control is performed through Supervisor's supported API;
- authorization is explicit, fail-closed and auditable;
- application code never needs to run as root merely because Supervisor is
  installed system-wide.

The rewrite must not preserve these implementation choices:

- authentication from `sctr.yml` password hashes;
- shell-like command construction through `pexpect`;
- parsing human-readable `supervisorctl status` output;
- authorization inferred only from a live PID;
- a root Supervisor socket treated as a per-user authorization boundary;
- user-writable configuration loaded by a root Supervisor instance;
- implicit or hardcoded socket paths;
- the current Python/Firenado lifecycle and packaging model.

### Recommended deployment decision

For personal applications, the safest default is one Supervisor instance per
Linux user, owned by that user and managed by `systemd --user`. The user then
controls a `0600` Unix socket and their own configuration without any root
capability.

A central SCTR broker controlling the system Supervisor can remain available
for administrator-managed applications, but it needs an explicit application
registry, an authentication provider, an authorization policy and a separate
privileged boundary. It must never infer ownership from a PID as its primary
policy.

## 2. Evidence boundary and repository state

The audit covered the tracked Python source, configuration examples, tests,
packaging metadata, the current Git history and the ignored source checkout
artifacts.

At audit time the checkout was on `develop` and had these pre-existing
uncommitted changes:

```text
requirements/basic.txt
sctr/app.py
sctr/conf/firenado.yml
```

Those changes were not rewritten by this audit. The audit document is a new
file and the README link is the only intended documentation integration.

The repository has no Go module, no Go implementation and no configured CI
workflow or Makefile. The current code is from 2021 and declares Python 3.6
through 3.9 classifiers.

Observed validation limitations:

- `python tests/runtests.py` failed in the audit environment because the import
  `from tests import management_test` resolved to an installed `tests` package
  instead of the repository package.
- `python -m unittest tests.runtests -v` ran zero tests because the runner only
  builds its suite under `if __name__ == "__main__"`.
- The audit environment did not have the repository's Python dependencies
  available to import `pexpect` or `setuptools`.
- `ruff check sctr tests` found existing unused-import findings.
- `pycodestyle sctr tests` found existing formatting findings, including a
  missing final newline in `sctr/app.py`.
- No Python source was changed by this audit, so the existing source was not
  auto-formatted.

## 3. Current architecture

```text
User
  |
  v
Firenado management command
  |
  +--> UserService
  |      +--> local sctr.yml users[]
  |      +--> SHA-512 crypt-style password validation
  |
  +--> CtlService
         +--> pexpect -> supervisorctl status
         +--> psutil -> inspect PID owner
         +--> pexpect -> supervisorctl restart <name>
                                |
                                v
                    shared Supervisor Unix socket
```

Important source locations:

| Path | Responsibility | Audit result |
|---|---|---|
| `sctr/management/commands.py` | Registers management commands | Only `user` and `proc` command groups are registered |
| `sctr/management/tasks.py` | CLI tasks and authentication decorators | Implements list/restart, not start/stop/create |
| `sctr/services.py` | User lookup and Supervisor control | Main authorization and process-control risk surface |
| `sctr/cli.py` | Authentication decorator | Uses local configured users, not LDAP |
| `sctr/conf/sctr.yml` | Runtime config | Ignored local file containing user credential material |
| `sctr/conf/sctr.yml_example` | Config example | Documents a default password in a comment |
| `sctr/app.py` | Firenado component and Unix-socket HTTP client | Hardcodes a socket and initializes an unused client |
| `sctr/conf/sctr.supervisord.ini` | Supervisor entry for SCTR | Runs the SCTR app as `fpiraz` |
| `setup.py` | Python package metadata | Packages only `sctr`, not its subpackages or runtime resources |
| `tests/management_test.py` | Test coverage | Tests only configuration loading |

## 4. Existing behavior inventory

### Implemented

- `sctr user list` lists the statically configured local users.
- `sctr user test` validates a statically configured password.
- `sctr proc list` invokes `supervisorctl status`, parses the output and returns
  processes whose current PID owner matches the authenticated username.
- `sctr proc restart <process>` first finds a matching visible process and then
  invokes `supervisorctl restart <process>`.
- A `FATAL` process is displayed without a PID because it has no live process.

### Not implemented or incomplete

- `proc start` is not registered.
- `proc stop` is not registered.
- Application creation is not implemented; `UserAddTask.run()` raises
  `NotImplementedError`.
- There is no application registry or ownership configuration.
- There is no LDAP authentication or group-based authorization.
- There is no documented installation or operational procedure.
- There is no audit log or action correlation ID.
- There is no timeout, cancellation contract or structured error model for
  Supervisor actions.
- There is no reliable release artifact containing all runtime packages and
  templates.

The README claims that non-privileged users can start and stop applications,
but the command registry currently exposes only list and restart. The rewrite
must choose the intended contract explicitly instead of inheriting the README
claim accidentally.

## 5. Findings

Severity uses the following meanings:

- **Critical:** likely direct compromise of a privileged boundary or broad
  unauthorized control.
- **High:** unauthorized process control, privilege escalation path, or a
  production-blocking correctness defect.
- **Medium:** material reliability, maintainability, deployment or audit gap.
- **Low:** hygiene issue that should still be corrected during the rewrite.

| ID | Severity | Finding | Evidence | Required direction |
|---|---|---|---|---|
| SEC-001 | High | The process authorization boundary is inferred from a live PID instead of an explicit application owner policy. | `CtlService.get_processes()` compares `psutil.Process(pid).username()` with the authenticated username. | Store an explicit owner per application. Treat live UID inspection as a defense-in-depth consistency check only. |
| SEC-002 | High | `FATAL` processes bypass the owner check. | The `FATAL` branch records the process without a PID, and `restart()` accepts any returned matching name. | A stopped or `FATAL` process must be authorized from the application registry, never from the absence of a PID. Add a regression test. |
| SEC-003 | High | The central SCTR process controls a shared Supervisor socket, so Supervisor's socket permission is not a per-application authorization boundary. | `sctr/app.py` hardcodes `/run/supervisor/supervisor.sock`; `CtlService` invokes the configured `supervisorctl`. | Prefer per-user Supervisor instances. For a central broker, isolate a privileged control plane and enforce an explicit allowlist before every action. |
| SEC-004 | High | User authentication is a static local YAML list, not LDAP and not an operational identity provider. | `UserService.by_username()` reads `app_component.conf['users']`; `authenticate()` validates the configured hash. | Replace with a provider-backed authentication contract. For LDAP, use TLS, a least-privilege bind identity, timeouts and fail-closed behavior. Do not use the directory Manager account. |
| SEC-005 | High | Control commands are built as strings and human-readable output is parsed. | `pexpect.run("%s status" % self.ctl_cmd)` and `pexpect.spawn("%s restart %s" % (self.ctl_cmd, process))`. | Do not invoke `supervisorctl` through a shell-like boundary. Call Supervisor XML-RPC over a Unix socket with typed requests and responses. |
| SEC-006 | High | The owner check and restart are separated by a race window. | `get_processes()` inspects a PID, then `restart()` invokes a second operation by name. | Authorize by stable application identity, validate the configured owner, and handle state changes and Supervisor faults atomically from the controller's point of view. |
| FUNC-001 | High | The implementation cannot reliably start or stop a user's application. | Only `list` and `restart` are registered; the restart parser assumes two output lines. | Define and implement `list`, `start`, `stop` and `restart` with structured state handling and explicit authorization. |
| FUNC-002 | Medium | Supervisor output parsing is fragile. | Lines are split on `\r\n` and two spaces; PID and uptime are extracted by positional string operations. | Use XML-RPC methods such as `getAllProcessInfo`, `startProcess` and `stopProcess`. |
| FUNC-003 | Medium | Errors and exit statuses are swallowed or reduced to empty results. | `pexpect.EOF` returns from `restart()`; failed status execution is not modeled. | Return typed errors with operation, app name, supervisor fault, retryability and safe operator message. |
| FUNC-004 | Medium | A `FATAL` process has no authoritative owner in the current data model. | The source comment admits that the FATAL path bypasses ownership because no process exists. | Add a registry that maps Supervisor application names to owners and capabilities. |
| OPS-001 | High | The root/global Supervisor configuration can become an indirect root execution boundary. | The example config relies on `user=...`; a missing or incorrect `user=` can make a program run as the Supervisor account. | Require an explicit non-root runtime identity for user-owned apps. Refuse deployment when the owner is absent, invalid or privileged. |
| OPS-002 | Medium | The runtime socket path is hardcoded and the configured CLI path is trusted as a command string. | `sctr/app.py` uses `/run/supervisor/supervisor.sock`; `sctr.yml` provides `supervisor.ctl`. | Make the transport endpoint explicit configuration, validate it, require an absolute Unix path and remove executable-path configuration from the control path. |
| OPS-003 | Medium | The HTTP client is dead or incomplete. | `SctrComponent.initialize()` creates `self.supervisor`, but no handler uses it; the only handler returns a static string. | Remove dead HTTP code or define a real API contract. Do not carry pycurl into the rewrite without a tested requirement. |
| SUP-001 | High | The package artifact omits runtime subpackages and resources. | `setup.py` uses `packages=["sctr"]`; the inspected sdist did not contain `sctr.management`, templates or config resources. | Build a reproducible Go artifact and test installation from a clean archive. |
| SUP-002 | Medium | `pexpect` is imported but not declared directly in `requirements/basic.txt`. | `sctr/services.py` imports `pexpect`; the direct requirements list only Firenado, pycurl and psutil. | Remove the dependency with the Go rewrite. For the legacy branch, declare every direct dependency before release. |
| SUP-003 | Medium | The package and runtime metadata are stale and inconsistent. | Python classifiers target 3.6–3.9; the ignored generated metadata still reports older dependency data; current source changes are uncommitted. | Define the Go version, module dependencies, release process and clean-build checks. |
| AUTH-001 | High | Credential material is embedded in local configuration examples and the default password is documented in comments. | `sctr/conf/sctr.yml_example` includes a default-password comment and a password hash. | Remove default credentials. Use LDAP/OIDC or a protected secret mechanism. Never print or commit secrets. |
| AUTH-002 | Medium | There is no lockout, rate limit, session policy, MFA boundary or audit record. | Authentication is a direct hash comparison in `UserService.authenticate()`. | Define the identity and session model before implementing a network API. |
| TEST-001 | High | Authorization has no tests for cross-user access, FATAL state, STOPPED state, PID reuse or Supervisor faults. | The only test verifies that a fixture configuration loads a user. | Make authorization tests a release gate and add a fake XML-RPC Supervisor boundary. |
| TEST-002 | Medium | The test runner is not reliably executable from a clean environment. | `tests/runtests.py` collides with an installed `tests` package; the audit environment also lacked required dependencies. | Use Go's standard test discovery and run it from a clean module/archive in CI. |
| DOC-001 | Medium | Installation, usage and operations are TODO in README. | `README.md` contains TODO entries for installation and usage. | Replace README with a short operator entry point linking to the Go design and runbooks. |

### Highest-priority blockers

Do not expose a Go replacement to real users until these are resolved:

1. Explicit application ownership and action authorization exist.
2. `FATAL` and `STOPPED` states are covered by tests.
3. The controller does not use shell command construction or output parsing.
4. Root-owned applications cannot be controlled by an unprivileged user unless
   an explicit reviewed policy says so.
5. Authentication no longer depends on a static password list.
6. The package is installable from a clean release artifact.

## 6. Threat model

### Assets

- Availability and integrity of each managed application.
- The host's root and service-account privileges.
- LDAP credentials and user identity data.
- Supervisor socket and XML-RPC control methods.
- Application configuration, logs and audit records.

### Attacker-controlled inputs

- Authenticated username and password or external identity assertion.
- Requested application name and action.
- Application source files when users own their applications.
- LDAP responses and network failures.
- Supervisor process states, fault messages and log contents.
- Configuration files if directory permissions are wrong.

### Trust boundaries

```text
Human/SSH/API client
        |
        | authentication and request validation
        v
SCTR controller
        |
        | explicit owner/action policy
        v
Supervisor XML-RPC Unix socket
        |
        | configured process command and runtime UID
        v
Managed application
```

The root Supervisor socket is a privileged boundary. A process that can issue
unrestricted XML-RPC methods against it can start, stop, reload or otherwise
control more than one user's application. Socket membership is therefore not a
substitute for SCTR authorization.

### Abuse cases and controls

| Abuse case | Control |
|---|---|
| User requests another user's app | Exact registry lookup by canonical name; compare requested action with owner and capability; return the same safe denial for unknown and unauthorized names. |
| User requests a `FATAL` app | Use the explicit registry owner; never infer permission from a missing PID. |
| User changes an app command or config | User-owned mode uses a user Supervisor. Managed mode keeps Supervisor configuration and registry root-owned and admin-controlled. |
| User controls a root process | Reject user-owned registration with a privileged runtime UID; validate runtime UID after start as defense in depth. |
| Malformed app name reaches Supervisor | Validate against the registry; never pass arbitrary names or wildcards to XML-RPC. |
| LDAP is unavailable | Fail closed for authentication and authorization; preserve existing local break-glass access for host administration. |
| LDAP bind credential leaks | Use a protected file or secret provider, TLS, no command-line password, no logs and rotation. |
| Supervisor returns a fault | Map the fault to a safe typed error; do not expose internal paths, commands or credentials. |
| Log or response contains secrets | Redact credentials and hashes; bound response sizes; do not copy Supervisor logs into audit records by default. |
| User abuses restart for denial of service | Per-user ownership, optional action rate limit and audit trail; administrator-defined restart policy. |

## 7. Target contract for the Go replacement

### User capabilities

The first stable contract should support:

```text
app list
app status <name>
app start <name>
app stop <name>
app restart <name>
```

Administrative provisioning is a separate capability:

```text
admin app validate <name>
admin app create <name> ...
admin app remove <name>
admin app reconcile
```

The exact CLI names may change, but the policy boundary must remain explicit.
User commands must never expose unrestricted Supervisor methods such as
`reloadConfig`, `addProcessGroup`, `removeProcessGroup`, `shutdown` or arbitrary
XML-RPC method dispatch.

### Application identity

An application is identified by a canonical exact name, not by a PID:

```text
<owner>/<application>
```

The Supervisor process name may be stored separately. The controller must
reject ambiguous names, wildcards and names that are not present in the
registry.

A registry entry needs at least:

```json
{
  "name": "marcuscosta-api",
  "owner": "marcuscosta",
  "runtime_user": "marcuscosta",
  "allowed_actions": ["list", "status", "start", "stop", "restart"],
  "supervisor_name": "marcuscosta-api",
  "enabled": true
}
```

The registry must be root-owned and not writable by application users in the
managed mode. Configuration drift must fail closed or be reported as an
explicit reconciliation error.

For user-owned Supervisor instances, the registry can be the user's own
configuration directory because the daemon and socket are owned by that user.
The system-wide controller must not load user-writable files as root.

### State semantics

Define these states and transitions before implementation:

| State | `list/status` | `start` | `stop` | `restart` |
|---|---|---|---|---|
| `RUNNING` | allowed | idempotent or explicit already-running result | allowed | allowed |
| `STOPPED` | allowed | allowed | idempotent or explicit already-stopped result | defined as start or rejected |
| `FATAL` | allowed | allowed after policy check | no-op or explicit result | allowed only by registry owner |
| `STARTING`/`STOPPING` | allowed with transitional state | return conflict or wait with timeout | return conflict or wait with timeout | return conflict or wait with timeout |
| unknown | safe not-found result | denied | denied | denied |

Every operation needs a context deadline. The controller must not wait
forever for a Supervisor transition.

### Authentication and authorization

Recommended profiles:

1. **User-owned local mode:** the Linux user reaches only their own
   `systemd --user` Supervisor socket. Authentication is provided by the OS
   session and Unix socket permissions; no application password is needed.
2. **Central managed mode:** SCTR authenticates a human through the approved
   identity provider, preferably LDAP over TLS or an existing SSO boundary.
   Authentication and application authorization are separate decisions.
3. **Administrator mode:** privileged operations run through a separate admin
   path, protected by host access policy and explicit operator identity.

The Go rewrite must not use `nslcd` as if it were an application authentication
API. `nslcd` is a local NSS/PAM daemon. If direct LDAP is selected for SCTR, use
a maintained LDAP client, TLS certificate validation, connection and bind
timeouts, a least-privilege reader/bind identity and secret-safe logging.

### Supervisor transport

Supervisor exposes an HTTP/XML-RPC API. The Go client should connect directly
to the configured Unix socket using a custom `http.Transport` dialer rather
than invoking `supervisorctl`.

The minimum API surface is:

```text
supervisor.getAllProcessInfo
supervisor.getProcessInfo
supervisor.startProcess
supervisor.stopProcess
```

The client must model XML-RPC success and fault responses explicitly. It must
validate response types, bound response sizes and return errors that preserve
`errors.Is`/`errors.As` behavior where useful.

Do not expose a generic `Call(method string, args ...any)` to user-facing code.
Keep the supported methods concrete and small. A generic internal transport
may exist only if it earns its place through multiple concrete methods and
focused tests.

### Configuration and permissions

Suggested managed-mode layout:

```text
/etc/sctr/
├── sctr.json                 # root:root, 0640 or stricter
├── applications.json         # root:root, 0640 or stricter
└── certs/                    # root-owned, policy-defined permissions

/run/sctr/
└── sctr.sock                 # root-owned, explicit group/mode if required
```

Suggested user-owned layout:

```text
~/.config/sctr/
├── supervisord.conf
└── apps/*.conf

~/.local/state/sctr/
└── logs/

~/.local/run/sctr/
└── supervisor.sock
```

Use absolute paths. Create runtime directories with the correct owner and
SELinux label where applicable. Never place a privileged control socket in a
world-writable directory such as `/tmp`.

## 8. Go package and process design

This is a behavior-oriented starting point, not a mandatory package tree:

```text
cmd/sctr/
internal/config/
internal/auth/
internal/policy/
internal/supervisor/
internal/transport/
internal/audit/
internal/cli/
```

Guidance:

- Keep concrete types close to the behavior that consumes them.
- Use `context.Context` on every operation that crosses a process, socket or
  network boundary.
- Use `net/http`, `net`, `encoding/xml`, `encoding/json`, `os`, `syscall` only
  where their behavior is sufficient before adding dependencies.
- Use `os/exec` only for administrative setup that genuinely needs an external
  program; never for routine Supervisor control.
- Do not create broad interfaces before a second real implementation or test
  seam exists.
- Keep the controller's happy path visible: validate, authorize, call
  Supervisor, map state, audit and return.
- Redact credentials, bind identities where sensitive, private paths and full
  command lines from errors and audit logs.
- Set bounded HTTP/XML-RPC request and response sizes.
- Close response bodies and stop timers explicitly.
- Use stable exit codes for CLI automation.

## 9. Test and validation plan

### Unit tests

Required before integration:

- canonical application-name validation;
- owner and action policy matrix;
- unknown, disabled and unauthorized applications;
- `RUNNING`, `STOPPED`, `FATAL`, `STARTING` and `STOPPING` behavior;
- XML-RPC request serialization and XML escaping;
- XML-RPC success responses for every supported method;
- XML-RPC fault responses and malformed XML;
- timeout, context cancellation and connection refusal;
- bounded response handling;
- audit redaction;
- config permissions and invalid configuration.

### Transport tests

Use a local Unix socket or a controlled `httptest` server with a custom dialer.
The fake Supervisor must assert the method name, arguments and response. It
must also return faults and delayed responses.

Do not mock away the XML-RPC boundary so completely that serialization, socket
selection and fault handling remain untested.

### Integration tests

Run against a disposable Supervisor configuration with applications that run
as non-root identities:

- Python application;
- Node/JavaScript application;
- Go binary;
- an application that exits immediately and becomes `FATAL`;
- an application that is initially `STOPPED`.

The language is not the authorization boundary. The runtime UID, command
ownership, configuration ownership and control socket are the relevant
security properties.

Prove all of the following:

- Marcuscosta can list and control only Marcuscosta applications;
- a stopped or FATAL Marcuscosta application remains controllable;
- Marcuscosta cannot control another user's application;
- Marcuscosta cannot control a root-owned application;
- a changed application command is not accepted from an unprivileged config
  directory in managed mode;
- LDAP failure denies new central actions without exposing a secret;
- local break-glass administration remains available;
- restart timeout and Supervisor faults are visible and safe.

### Go quality gates

No repository-specific Go gates exist yet. The conventional baseline for the
rewrite is:

```bash
gofmt -w <touched-go-files>
goimports -w <touched-go-files>  # when available
go test ./...
go vet ./...
go test -race ./...           # when concurrency is introduced
```

The release check must also build from a clean checkout and verify the output:

```bash
go build ./cmd/sctr
```

## 10. Migration sequence

1. **Freeze the contract.** Decide whether start/stop are part of the first
   release, define user-owned versus managed mode and document state semantics.
2. **Inventory existing applications.** Record Supervisor names, runtime UIDs,
   config owners, command paths, log paths and current administrators. Do not
   copy credentials into the inventory.
3. **Implement the XML-RPC transport.** Prove Unix-socket connection, typed
   requests, faults, timeouts and cancellation against a fake Supervisor.
4. **Implement policy.** Add explicit application ownership, allowed actions,
   privileged-runtime rejection and fail-closed drift handling.
5. **Implement identity.** Add the approved LDAP/SSO integration for central
   mode. Keep authentication separate from application policy.
6. **Implement the CLI.** Add stable list/status/start/stop/restart commands
   with safe errors and exit codes.
7. **Implement user-owned mode.** Add a documented `systemd --user` unit and
   per-user socket/config layout. Require administrator-enabled linger only
   when persistent post-logout execution is an intentional policy.
8. **Implement managed mode.** Add root-owned registry/configuration and an
   administrator-only provisioning path. Never include user-writable config in
   the root Supervisor process.
9. **Run the disposable integration matrix.** Include Python, JavaScript and Go
   apps plus STOPPED/FATAL and cross-user cases.
10. **Parallel rollout.** Run the Go controller beside Python SCTR in read-only
    mode, compare visible application sets and audit decisions, then enable
    actions for one test user.
11. **Cut over.** Revoke Python SCTR access to the shared socket, deploy the Go
    binary from a clean artifact, retain rollback configuration and monitor
    authorization denials and Supervisor faults.
12. **Retire the legacy path.** Remove static passwords, old pycurl code,
    pexpect control and undocumented root socket access only after rollback is
    no longer required.

## 11. Handoff checklist for the next agent

Before implementation begins, confirm these decisions in Taskwarrior and in the
Go plan:

- [ ] Is the first release user-owned Supervisor, managed Supervisor or both?
- [ ] Are `start`, `stop`, `restart`, `status` and `list` all in scope?
- [ ] Which identity provider is authoritative for central mode?
- [ ] Which exact applications belong to each user?
- [ ] Which runtime UIDs are permitted, and is root always rejected for user apps?
- [ ] Where is the root-owned application registry stored?
- [ ] Which Supervisor XML-RPC methods are allowed?
- [ ] What are the timeout, retry and rate-limit policies?
- [ ] What audit events are required and what fields must be redacted?
- [ ] How is rollback performed if the Go controller cannot connect?
- [ ] Which package/release pipeline builds and verifies the binary?

The implementation agent should start with the transport and authorization test
matrix, not with a command wrapper or a web UI.

## 12. References

### Supervisor

- [Supervisor XML-RPC API](https://supervisord.org/api.html)
- [Supervisor configuration file](https://supervisord.org/configuration.html)
- [Supervisor running and process control](https://supervisord.org/running.html)
- [Supervisor subprocesses and runtime users](https://supervisord.org/subprocess.html)

### Go standard library

- [`net/http.Transport`](https://pkg.go.dev/net/http#Transport)
- [`net.Dialer.DialContext`](https://pkg.go.dev/net#Dialer.DialContext)
- [`encoding/xml`](https://pkg.go.dev/encoding/xml)
- [`encoding/json`](https://pkg.go.dev/encoding/json)
- [`os/exec`](https://pkg.go.dev/os/exec)
- [`context`](https://pkg.go.dev/context)

### Identity and security

- [RHEL 8: migrating authentication from nslcd to SSSD](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/8/html/configuring_authentication_and_authorization_in_rhel/assembly_migrating-authentication-from-nslcd-to-sssd_configuring-authentication-and-authorization-in-rhel)
- [OWASP A01: Broken Access Control](https://owasp.org/Top10/A01_2021-Broken_Access_Control/)
- [RHEL: troubleshooting SELinux problems](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/8/html/using_selinux/troubleshooting-problems-related-to-selinux)

## Final security rule

If the Go rewrite cannot prove which user owns an application and which runtime
identity Supervisor will use, it must refuse the action. A useful error is
safer than a convenient root process.
