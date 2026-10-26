use std::io::{self, Write};

use crate::Engine;

pub(crate) fn write_accounts<W: Write>(out: &mut W, engine: &Engine) -> io::Result<()> {
    writeln!(out, "client,available,held,total,locked")?;
    for (client, account) in engine.accounts() {
        writeln!(
            out,
            "{},{},{},{},{}",
            client.get(),
            account.available(),
            account.held(),
            account.total(),
            account.locked()
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
