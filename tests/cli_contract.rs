use std::process::{Command, Output};

fn sctr(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sctr"))
        .args(args)
        .output()
        .expect("run sctr")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

#[test]
fn empty_help_command_and_help_flag_match_exactly() {
    let empty = sctr(&[]);
    let command = sctr(&["help"]);
    let flag = sctr(&["--help"]);

    assert!(empty.status.success(), "{}", stderr(&empty));
    assert!(command.status.success(), "{}", stderr(&command));
    assert!(flag.status.success(), "{}", stderr(&flag));
    assert_eq!(stdout(&empty), stdout(&command));
    assert_eq!(stdout(&command), stdout(&flag));
}

#[test]
fn root_help_is_an_operational_briefing() {
    let output = sctr(&["help"]);
    let output = stdout(&output);

    for section in [
        "ROLE",
        "CONTROL FLOW",
        "SECURITY BOUNDARY",
        "COMMANDS",
        "AGENT RULES",
        "NEXT",
        "GLOBAL OPTIONS",
    ] {
        assert!(output.contains(section), "missing {section}: {output}");
    }
    assert_eq!(output.matches("  app          ").count(), 1);
    assert_eq!(output.matches("  help         ").count(), 1);
}

#[test]
fn nested_help_uses_the_same_command_tree() {
    let explicit = sctr(&["help", "app", "start"]);
    let flag = sctr(&["app", "start", "--help"]);

    assert!(explicit.status.success(), "{}", stderr(&explicit));
    assert!(flag.status.success(), "{}", stderr(&flag));
    assert_eq!(stdout(&explicit), stdout(&flag));

    let help = stdout(&explicit);
    for section in [
        "USAGE",
        "ROLE",
        "AVAILABILITY",
        "PREREQUISITES",
        "SIDE EFFECTS AND OUTPUT",
        "EXAMPLES",
        "NEXT ACTION",
    ] {
        assert!(help.contains(section), "missing {section}: {help}");
    }
}

#[test]
fn planned_lifecycle_operations_fail_without_side_effects() {
    let output = sctr(&["app", "start", "api"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let error = stderr(&output);
    assert!(error.contains("ERROR:"), "{error}");
    assert!(error.contains("ACTION:"), "{error}");
    assert!(error.contains("not implemented"), "{error}");
}

#[test]
fn unknown_commands_return_actionable_usage_errors() {
    let output = sctr(&["systemctl", "restart", "anything"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    let error = stderr(&output);
    assert!(error.contains("unknown command"), "{error}");
    assert!(error.contains("ACTION: Run 'sctr help'"), "{error}");
}
