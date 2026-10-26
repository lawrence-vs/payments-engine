use std::io::{self, Write};

use crate::Engine;

pub(crate) fn write_accounts<W: Write>(out: &mut W, engine: &Engine) -> io::Result<()> {
    writeln!(out, "client,available,held,total,locked")?;
    for (client, account) in engine.accounts() {
        let Some(total) = account.total() else {
            let message = format!("client {}: available plus held does not fit", client.get());
            return Err(io::Error::new(io::ErrorKind::InvalidData, message));
        };
        writeln!(
            out,
            "{},{},{},{},{}",
            client.get(),
            account.available(),
            account.held(),
            total,
            account.locked()
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
