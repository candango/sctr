#![forbid(unsafe_code)]

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();

    let code = sctr::run(env::args_os().skip(1), &mut stdout, &mut stderr);
    let _ = stdout.flush();
    let _ = stderr.flush();

    ExitCode::from(code)
}
