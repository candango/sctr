# SCTR command-line contract

## Purpose

The SCTR CLI is a policy client, not a generic systemd frontend. Its help is an
operational briefing: it explains the application-oriented command surface,
the security boundary, expected effects, and the next valid action.

The initial executable slice implements the command tree, registry
authorization, and typed systemd lifecycle adapter. List, status, start, stop,
and restart are operational when the root-owned registry at
`/etc/sctr/registry.json` and the systemd system manager are available. Logs
remain contract-only until the bounded journal adapter exists.

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
Each implemented lifecycle command resolves the current actor and application
through the root-owned registry, authorizes the exact action, invokes the exact
registered unit, and awaits the systemd result.

## Output and failures

Help and successful report output go to stdout. Failures go to stderr using the
stable shape:

```text
ERROR: <what failed>
ACTION: <next valid action>
```

Unknown syntax exits with status `2`. A known operation whose registry,
authorization, or systemd boundary is unavailable exits with status `1` and
fails closed. Neither condition reports a false success.
