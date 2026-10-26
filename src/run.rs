use std::io::{self, Read, Write};

pub fn run<R: Read, W: Write, E: Write>(_input: R, _out: W, _err: E) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
