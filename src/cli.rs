use std::fmt::Write as _;

use crate::{EXIT_FAILURE, EXIT_USAGE};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Availability {
    Available,
    ContractOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behavior {
    Help,
    Group,
    Lifecycle,
}

#[derive(Clone, Copy, Debug)]
struct CommandSpec {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    role: &'static str,
    availability: Availability,
    prerequisites: &'static [&'static str],
    effects: &'static [&'static str],
    examples: &'static [&'static str],
    next_action: &'static str,
    children: &'static [CommandSpec],
    behavior: Behavior,
}

#[derive(Clone, Copy, Debug)]
struct CommandGroup {
    name: &'static str,
    commands: &'static [CommandSpec],
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CliError {
    pub(crate) exit_code: u8,
    pub(crate) message: String,
    pub(crate) action: String,
}

impl CliError {
    fn usage(message: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_USAGE,
            message: message.into(),
            action: action.into(),
        }
    }

    fn unavailable(message: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_FAILURE,
            message: message.into(),
            action: action.into(),
        }
    }
}

const NO_CHILDREN: &[CommandSpec] = &[];

const APP_LIST: CommandSpec = CommandSpec {
    name: "list",
    summary: "List applications visible to the authenticated tenant",
    usage: "sctr app list",
    role: "Resolve the authenticated tenant and list only applications owned by that tenant.",
    availability: Availability::ContractOnly,
    prerequisites: &[
        "An authenticated local or service identity must resolve to one tenant.",
        "The root-owned application registry must be available.",
    ],
    effects: &[
        "Reads tenant-scoped registry entries without contacting arbitrary systemd units.",
        "Prints application identifiers rather than internal unit names.",
    ],
    examples: &["sctr app list"],
    next_action: "Choose a returned application identifier and run 'sctr app status <app>'.",
    children: NO_CHILDREN,
    behavior: Behavior::Lifecycle,
};

const APP_STATUS: CommandSpec = CommandSpec {
    name: "status",
    summary: "Show status for one registered application",
    usage: "sctr app status <app>",
    role: "Resolve one tenant-owned application to its exact unit and return scoped service state.",
    availability: Availability::ContractOnly,
    prerequisites: &[
        "The application identifier must be registered to the authenticated tenant.",
        "The systemd adapter must be available with a bounded deadline.",
    ],
    effects: &[
        "Reads approved unit and service properties for the resolved application.",
        "Does not accept a unit name, PID, UID, path, or D-Bus method from the client.",
    ],
    examples: &["sctr app status api"],
    next_action: "Use start, stop, or restart only when the returned state requires it.",
    children: NO_CHILDREN,
    behavior: Behavior::Lifecycle,
};

const APP_START: CommandSpec = CommandSpec {
    name: "start",
    summary: "Start one registered application",
    usage: "sctr app start <app>",
    role: "Authorize a tenant application and start its exact registered systemd unit.",
    availability: Availability::ContractOnly,
    prerequisites: &[
        "The application must belong to the authenticated tenant and allow start.",
        "The registry must map it to one exact root-owned unit.",
    ],
    effects: &[
        "Queues StartUnit for the exact registered unit and awaits the matching job result.",
        "Reports failure when the job or resulting service state fails.",
    ],
    examples: &["sctr app start api"],
    next_action: "Run 'sctr app status <app>' after a successful start.",
    children: NO_CHILDREN,
    behavior: Behavior::Lifecycle,
};

const APP_STOP: CommandSpec = CommandSpec {
    name: "stop",
    summary: "Stop one registered application",
    usage: "sctr app stop <app>",
    role: "Authorize a tenant application and stop its complete systemd service cgroup.",
    availability: Availability::ContractOnly,
    prerequisites: &[
        "The application must belong to the authenticated tenant and allow stop.",
        "The registry must map it to one exact root-owned unit.",
    ],
    effects: &[
        "Queues StopUnit for the exact registered unit and awaits the matching job result.",
        "Does not signal an arbitrary PID or accept an arbitrary unit name.",
    ],
    examples: &["sctr app stop worker"],
    next_action: "Run 'sctr app status <app>' to verify the stopped state.",
    children: NO_CHILDREN,
    behavior: Behavior::Lifecycle,
};

const APP_RESTART: CommandSpec = CommandSpec {
    name: "restart",
    summary: "Restart one registered application",
    usage: "sctr app restart <app>",
    role: "Authorize a tenant application and restart its exact registered systemd unit.",
    availability: Availability::ContractOnly,
    prerequisites: &[
        "The application must belong to the authenticated tenant and allow restart.",
        "The registry must map it to one exact root-owned unit.",
    ],
    effects: &[
        "Queues RestartUnit for the exact registered unit and awaits the matching job result.",
        "Does not perform daemon-reload or mutate the root-owned unit contract.",
    ],
    examples: &["sctr app restart api"],
    next_action: "Run 'sctr app status <app>' and the application health check.",
    children: NO_CHILDREN,
    behavior: Behavior::Lifecycle,
};

const APP_LOGS: CommandSpec = CommandSpec {
    name: "logs",
    summary: "Read bounded logs for one registered application",
    usage: "sctr app logs <app>",
    role: "Resolve one tenant-owned application and read logs scoped to its exact unit.",
    availability: Availability::ContractOnly,
    prerequisites: &[
        "The application identifier must be registered to the authenticated tenant.",
        "The logging adapter must enforce time and size bounds.",
    ],
    effects: &[
        "Reads journal records selected by the resolved unit identity.",
        "Does not accept an arbitrary journal query or filesystem path.",
    ],
    examples: &["sctr app logs api"],
    next_action: "Use the scoped records to diagnose the application, then re-check status.",
    children: NO_CHILDREN,
    behavior: Behavior::Lifecycle,
};

const APP_OPERATIONS: &[CommandSpec] = &[
    APP_LIST,
    APP_STATUS,
    APP_START,
    APP_STOP,
    APP_RESTART,
    APP_LOGS,
];

const APP_COMMAND: CommandSpec = CommandSpec {
    name: "app",
    summary: "Inspect and control registered tenant applications",
    usage: "sctr app <list|status|start|stop|restart|logs> [<app>]",
    role: "Expose the narrow application lifecycle contract without becoming a generic systemd proxy.",
    availability: Availability::Available,
    prerequisites: &[
        "The caller identity must be established outside untrusted request fields.",
        "Lifecycle operations require a root-owned registry and an independently validated unit mapping.",
    ],
    effects: &[
        "Accepts application identifiers only; internal units and runtime identities remain server-owned.",
        "Delegates only typed, authorized operations after the lifecycle implementation is available.",
    ],
    examples: &["sctr app list", "sctr help app restart"],
    next_action: "Run 'sctr help app <operation>' before invoking a lifecycle operation.",
    children: APP_OPERATIONS,
    behavior: Behavior::Group,
};

const HELP_COMMAND: CommandSpec = CommandSpec {
    name: "help",
    summary: "Show the SCTR operational command briefing",
    usage: "sctr help [command [operation]]",
    role: "Explain the command contract, security boundary, side effects, and next valid action before execution.",
    availability: Availability::Available,
    prerequisites: &[],
    effects: &["Reads static command metadata and never changes application or system state."],
    examples: &["sctr help", "sctr help app", "sctr help app start"],
    next_action: "Run 'sctr help app' to inspect the tenant application surface.",
    children: NO_CHILDREN,
    behavior: Behavior::Help,
};

const APPLICATION_COMMANDS: &[CommandSpec] = &[APP_COMMAND];
const REFERENCE_COMMANDS: &[CommandSpec] = &[HELP_COMMAND];

const ROOT_GROUPS: &[CommandGroup] = &[
    CommandGroup {
        name: "manage tenant applications",
        commands: APPLICATION_COMMANDS,
    },
    CommandGroup {
        name: "inspect the command contract",
        commands: REFERENCE_COMMANDS,
    },
];

pub(crate) fn dispatch(args: &[String]) -> Result<String, CliError> {
    if args.is_empty() {
        return Ok(render_root_help());
    }

    match args[0].as_str() {
        "-h" | "--help" if args.len() == 1 => Ok(render_root_help()),
        "-V" | "--version" if args.len() == 1 => {
            Ok(format!("sctr {}\n", env!("CARGO_PKG_VERSION")))
        }
        option if option.starts_with('-') => Err(CliError::usage(
            format!("unknown global option {option:?}"),
            "Run 'sctr help' to inspect supported global options.",
        )),
        _ => dispatch_command(args),
    }
}

fn render_help(path: &[String]) -> Result<String, CliError> {
    if path.is_empty() {
        return Ok(render_root_help());
    }

    let command = find_command(path).ok_or_else(|| {
        let topic = path.join(" ");
        let parent = if path.len() > 1 {
            format!("sctr help {}", path[..path.len() - 1].join(" "))
        } else {
            "sctr help".to_owned()
        };
        CliError::usage(
            format!("unknown help topic {topic:?}"),
            format!("Run '{parent}' to inspect valid commands."),
        )
    })?;

    Ok(render_command_help(path, command))
}

fn dispatch_command(args: &[String]) -> Result<String, CliError> {
    if matches!(args.last().map(String::as_str), Some("-h" | "--help")) {
        return render_help(&args[..args.len() - 1]);
    }

    let Some(command) = find_root_command(&args[0]) else {
        return Err(CliError::usage(
            format!("unknown command {:?}", args[0]),
            "Run 'sctr help' to inspect the application-oriented command surface.",
        ));
    };

    match command.behavior {
        Behavior::Help => render_help(&args[1..]),
        Behavior::Group => dispatch_group(command, args),
        Behavior::Lifecycle => dispatch_lifecycle(command, &args[1..], &[command.name]),
    }
}

fn dispatch_group(command: &CommandSpec, args: &[String]) -> Result<String, CliError> {
    let Some(operation_name) = args.get(1) else {
        return Err(CliError::usage(
            format!("{} requires an operation", command.name),
            format!(
                "Run 'sctr help {}' to inspect valid operations.",
                command.name
            ),
        ));
    };

    let Some(operation) = find_child(command, operation_name) else {
        return Err(CliError::usage(
            format!("unknown {} operation {operation_name:?}", command.name),
            format!(
                "Run 'sctr help {}' to inspect valid operations.",
                command.name
            ),
        ));
    };

    dispatch_lifecycle(operation, &args[2..], &[command.name, operation.name])
}

fn dispatch_lifecycle(
    command: &CommandSpec,
    _args: &[String],
    path: &[&str],
) -> Result<String, CliError> {
    let command_path = path.join(" ");
    let message = match command.availability {
        Availability::Available => format!("'sctr {command_path}' has no lifecycle handler"),
        Availability::ContractOnly => {
            format!("'sctr {command_path}' is not implemented in this bootstrap slice")
        }
    };

    Err(CliError::unavailable(
        message,
        format!(
            "Run 'sctr help {command_path}' to inspect its contract; implement the registry policy and typed systemd boundary before enabling it."
        ),
    ))
}

fn find_command(path: &[String]) -> Option<&'static CommandSpec> {
    let mut command = find_root_command(path.first()?)?;
    for segment in &path[1..] {
        command = find_child(command, segment)?;
    }
    Some(command)
}

fn find_root_command(name: &str) -> Option<&'static CommandSpec> {
    ROOT_GROUPS
        .iter()
        .flat_map(|group| group.commands.iter())
        .find(|command| command.name == name)
}

fn find_child(command: &CommandSpec, name: &str) -> Option<&'static CommandSpec> {
    command.children.iter().find(|child| child.name == name)
}

macro_rules! line {
    ($output:expr) => {
        $output.push('\n')
    };
    ($output:expr, $($argument:tt)*) => {
        append_line($output, format_args!($($argument)*))
    };
}

fn append_line(output: &mut String, arguments: std::fmt::Arguments<'_>) {
    // Invariant: fmt::Write for String is infallible.
    let _ = output.write_fmt(arguments);
    output.push('\n');
}

fn render_root_help() -> String {
    let mut output = String::new();
    line!(
        &mut output,
        "sctr — policy-controlled tenant application lifecycle"
    );
    line!(&mut output);
    line!(&mut output, "ROLE");
    line!(
        &mut output,
        "  Resolve registered applications and expose a narrow lifecycle boundary over systemd."
    );
    line!(
        &mut output,
        "  The bootstrap command tree is available; lifecycle execution remains disabled until its policy and adapter exist."
    );
    line!(&mut output);
    line!(&mut output, "CONTROL FLOW");
    line!(
        &mut output,
        "  authenticate actor → resolve tenant/app → authorize action → exact systemd unit → await result"
    );
    line!(&mut output);
    line!(&mut output, "SECURITY BOUNDARY");
    line!(
        &mut output,
        "  Clients identify an application; they never provide a unit, UID, command, path, or D-Bus method."
    );
    line!(&mut output);
    line!(&mut output, "COMMANDS");
    for group in ROOT_GROUPS {
        line!(&mut output, "{}", group.name);
        for command in group.commands {
            line!(&mut output, "  {:<12} {}", command.name, command.summary);
        }
        line!(&mut output);
    }
    line!(&mut output, "AGENT RULES");
    line!(
        &mut output,
        "  Treat ERROR and ACTION lines as operational guidance. Never substitute a raw systemd target."
    );
    line!(
        &mut output,
        "  Use 'sctr help <command> [operation]' for prerequisites, effects, and the next action."
    );
    line!(&mut output);
    line!(&mut output, "NEXT");
    line!(
        &mut output,
        "  Run 'sctr help app' to inspect the application lifecycle contract."
    );
    line!(&mut output);
    line!(&mut output, "GLOBAL OPTIONS");
    line!(&mut output, "  -V, --version        Show version");
    line!(&mut output, "  -h, --help           Show this help message");
    output
}

fn render_command_help(path: &[String], command: &CommandSpec) -> String {
    let command_path = path.join(" ");
    let mut output = String::new();
    line!(&mut output, "sctr {command_path} — {}", command.summary);
    line!(&mut output);
    line!(&mut output, "USAGE");
    line!(&mut output, "  {}", command.usage);
    line!(&mut output);
    line!(&mut output, "ROLE");
    line!(&mut output, "  {}", command.role);
    line!(&mut output);
    line!(&mut output, "AVAILABILITY");
    match command.availability {
        Availability::Available => {
            line!(&mut output, "  Available in the current executable.");
        }
        Availability::ContractOnly => {
            line!(
                &mut output,
                "  Contract defined; execution is disabled until the policy and systemd adapter are implemented."
            );
        }
    }
    line!(&mut output);
    write_list(&mut output, "PREREQUISITES", command.prerequisites);
    write_list(&mut output, "SIDE EFFECTS AND OUTPUT", command.effects);
    if !command.children.is_empty() {
        line!(&mut output, "OPERATIONS");
        for child in command.children {
            let marker = match child.availability {
                Availability::Available => "available",
                Availability::ContractOnly => "contract only",
            };
            line!(
                &mut output,
                "  {:<12} {} ({marker})",
                child.name,
                child.summary
            );
        }
        line!(&mut output);
    }
    write_list(&mut output, "EXAMPLES", command.examples);
    line!(&mut output, "NEXT ACTION");
    line!(&mut output, "  {}", command.next_action);
    output
}

fn write_list(output: &mut String, heading: &str, values: &[&str]) {
    if values.is_empty() {
        return;
    }

    line!(output, "{heading}");
    for value in values {
        line!(output, "  - {value}");
    }
    line!(output);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn command_tree_has_unique_sibling_names() {
        fn assert_unique(commands: &[CommandSpec]) {
            let mut names = HashSet::new();
            for command in commands {
                assert!(
                    names.insert(command.name),
                    "duplicate command {}",
                    command.name
                );
                assert_unique(command.children);
            }
        }

        let root_commands: Vec<_> = ROOT_GROUPS
            .iter()
            .flat_map(|group| group.commands.iter().copied())
            .collect();
        assert_unique(&root_commands);
    }

    #[test]
    fn every_command_has_operational_help_metadata() {
        fn assert_metadata(command: &CommandSpec) {
            assert!(!command.summary.is_empty(), "{} summary", command.name);
            assert!(!command.usage.is_empty(), "{} usage", command.name);
            assert!(!command.role.is_empty(), "{} role", command.name);
            assert!(!command.effects.is_empty(), "{} effects", command.name);
            assert!(!command.examples.is_empty(), "{} examples", command.name);
            assert!(
                !command.next_action.is_empty(),
                "{} next action",
                command.name
            );
            for child in command.children {
                assert_metadata(child);
            }
        }

        for command in ROOT_GROUPS.iter().flat_map(|group| group.commands) {
            assert_metadata(command);
        }
    }

    #[test]
    fn lifecycle_commands_are_fail_closed() {
        for operation in APP_OPERATIONS {
            assert_eq!(operation.availability, Availability::ContractOnly);
            assert_eq!(operation.behavior, Behavior::Lifecycle);
        }
    }
}
