# SCTR command-line contract

## Purpose

The SCTR CLI is a policy client, not a generic systemd frontend. Its help is an
operational briefing: it explains the application-oriented command surface,
the security boundary, expected effects, and the next valid action.

The initial executable slice implements the command tree and help renderer. The
lifecycle operations are visible as accepted contracts but fail closed with an
actionable `not implemented` error until registry authorization and the systemd
adapter exist. This prevents a bootstrap binary from pretending that a queued
or unsupported operation succeeded.

## Navigation

These invocations render the same root briefing:

```text
sctr
sctr help
sctr --help
```

Use a command path for progressively narrower help:

```text
sctr help app
sctr help app start
sctr app start --help
```

The command tree is the source of truth for routing and help metadata. Each
entry carries its summary, usage, role, availability, prerequisites, effects,
examples, next action, and child operations. The renderer does not delegate the
agent-facing interface to a parser library.

## Application operations

```text
sctr app list
sctr app status <app>
sctr app start <app>
sctr app stop <app>
sctr app restart <app>
sctr app logs <app>
```

Clients supply a registered application identifier. They never supply a unit
name, UID, command, path, socket, journal query, or arbitrary D-Bus method.
Lifecycle commands must remain unavailable until the implementation can resolve
the authenticated tenant and application through the root-owned registry,
authorize the exact action, invoke the exact registered unit, and await the
systemd result.

## Output and failures

Help and successful report output go to stdout. Failures go to stderr using the
stable shape:

```text
ERROR: <what failed>
ACTION: <next valid action>
```

Unknown syntax exits with status `2`. A known operation whose implementation is
not available exits with status `1`. Neither condition mutates workflow or
system state.
