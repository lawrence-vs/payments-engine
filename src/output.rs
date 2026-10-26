use std::io::{self, Write};

use crate::Engine;

pub(crate) fn write_accounts<W: Write>(out: &mut W, engine: &Engine) -> io::Result<()> {
    let rows = engine
        .accounts()
        .map(|(client, account)| {
            account.total().map(|total| (client, account, total)).ok_or_else(|| {
                let message = format!("client {}: available plus held does not fit", client.get());
                io::Error::new(io::ErrorKind::InvalidData, message)
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    writeln!(out, "client,available,held,total,locked")?;
    for (client, account, total) in rows {
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
