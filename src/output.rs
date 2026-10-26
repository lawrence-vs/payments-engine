use std::io::{self, Write};

use crate::Engine;

pub(crate) fn write_accounts<W: Write>(_out: &mut W, _engine: &Engine) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
