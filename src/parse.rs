use std::fmt;
use std::io::{self, Read};

use crate::{AmountError, Event, TxId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Type,
    Client,
    Tx,
    Amount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnknownType { row: u64 },
    MissingField { row: u64, field: Field },
    TooManyFields { row: u64 },
    NotUtf8 { row: u64, field: Field },
    BadClient { row: u64 },
    BadTx { row: u64 },
    BadAmount { row: u64, reason: AmountError },
    ZeroAmount { row: u64 },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("parse error")
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    error: ParseError,
    burn: Option<TxId>,
}

impl Rejected {
    pub fn error(&self) -> &ParseError {
        &ParseError::UnknownType { row: 0 }
    }

    pub fn burn(&self) -> Option<TxId> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Event(Event),
    Rejected(Rejected),
    MissingHeader,
}

pub struct Events<R> {
    _reader: R,
}

impl<R: Read> Iterator for Events<R> {
    type Item = Result<Row, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub fn read_events<R: Read>(reader: R) -> Events<R> {
    Events { _reader: reader }
}

#[cfg(test)]
fn parse_record(row: u64, _record: &csv::ByteRecord) -> Result<Event, Rejected> {
    Err(Rejected { error: ParseError::UnknownType { row }, burn: None })
}

#[cfg(test)]
mod tests;
