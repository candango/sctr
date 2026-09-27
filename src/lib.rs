#![forbid(unsafe_code)]

mod cli;
mod registry;
mod runtime;
mod systemd;

use std::ffi::OsString;
use std::io::Write;

pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_FAILURE: u8 = 1;
pub const EXIT_USAGE: u8 = 2;

/// Runs one SCTR command against caller-provided streams.
///
/// The process boundary owns environment resolution and exit handling. This
/// function keeps command routing testable without mutating process-global
/// state.
pub fn run<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    run_with_dispatch(args, stdout, stderr, cli::dispatch)
}

/// Runs one SCTR command with the live registry and systemd adapter.
///
/// The process boundary owns runtime initialization while command routing stays
/// testable through [`run`].
pub fn run_system<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    run_with_dispatch(args, stdout, stderr, cli::dispatch_system)
}

fn run_with_dispatch<I, T, F>(
    args: I,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    dispatch: F,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    F: Fn(&[String]) -> Result<String, cli::CliError>,
{
    let mut parsed = Vec::new();
    for argument in args {
        let argument = argument.into();
        let Some(argument) = argument.to_str() else {
            return write_error(
                stderr,
                EXIT_USAGE,
                "command arguments must be valid UTF-8",
                "Use ASCII command names and a valid registered application identifier.",
            );
        };
        parsed.push(argument.to_owned());
    }

    match dispatch(&parsed) {
        Ok(output) => match stdout.write_all(output.as_bytes()) {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => write_error(
                stderr,
                EXIT_FAILURE,
                &format!("write command output: {error}"),
                "Check the output stream and retry the command.",
            ),
        },
        Err(error) => write_error(stderr, error.exit_code, &error.message, &error.action),
    }
}

fn write_error(stderr: &mut dyn Write, exit_code: u8, message: &str, action: &str) -> u8 {
    let _ = writeln!(stderr, "ERROR: {message}");
    let _ = writeln!(stderr, "ACTION: {action}");
    exit_code
}
