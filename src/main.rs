//! Binary: checks the one path argument, opens it, runs the library, maps the result to an exit code.

use std::fmt::Arguments;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        report(format_args!("usage: {} <input.csv>", env!("CARGO_BIN_NAME")));
        return ExitCode::from(2);
    };
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            report(format_args!("cannot open {}: {error}", Path::new(&path).display()));
            return ExitCode::from(1);
        }
    };
    match payments_engine::run(file, BufWriter::new(io::stdout().lock()), io::stderr().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(format_args!("error: {error}"));
            ExitCode::from(1)
        }
    }
}

// A closed stderr must not change the exit code.
fn report(message: Arguments) {
    let _ = writeln!(io::stderr().lock(), "{message}");
}
