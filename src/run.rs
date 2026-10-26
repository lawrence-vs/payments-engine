use std::io::{self, Read, Write};

use crate::diagnostics::{Category, Diagnostics, category};
use crate::output::write_accounts;
use crate::{Engine, Row, read_events};

pub fn run<R: Read, W: Write, E: Write>(input: R, mut out: W, err: E) -> io::Result<()> {
    let mut engine = Engine::new();
    let mut diagnostics = Diagnostics::new(err);
    for item in read_events(input) {
        match item? {
            Row::MissingHeader => {
                diagnostics.warn("first line is not a header, read as data");
            }
            Row::Rejected(rejected) => {
                diagnostics.reject(Category::Bad, rejected.error());
                if let Some(tx) = rejected.burn() {
                    engine.burn(tx);
                }
            }
            Row::Event { row, event } => {
                if let Err(error) = engine.apply(event) {
                    diagnostics.reject(category(&error), format_args!("row {row}: {error}"));
                }
            }
        }
    }
    write_accounts(&mut out, &engine)?;
    out.flush()?;
    diagnostics.finish();
    Ok(())
}

#[cfg(test)]
mod tests;
