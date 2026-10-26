use std::fmt;
use std::io::{self, Read};
use std::str::FromStr;

use csv::{ByteRecord, ByteRecordsIntoIter, ReaderBuilder, Trim};

use crate::{Amount, AmountError, ClientId, Event, TxId};

const HEADER: [&str; 4] = ["type", "client", "tx", "amount"];
const KINDS: [(&str, Kind); 5] = [
    ("deposit", Kind::Deposit),
    ("withdrawal", Kind::Withdrawal),
    ("dispute", Kind::Dispute),
    ("resolve", Kind::Resolve),
    ("chargeback", Kind::Chargeback),
];
const MAX_FIELDS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Type,
    Client,
    Tx,
    Amount,
}

impl Field {
    fn name(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Client => "client",
            Self::Tx => "tx",
            Self::Amount => "amount",
        }
    }
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
        match self {
            Self::UnknownType { row } => write!(f, "row {row}: unknown event type"),
            Self::MissingField { row, field } => write!(f, "row {row}: missing {}", field.name()),
            Self::TooManyFields { row } => write!(f, "row {row}: a value after field {MAX_FIELDS}"),
            Self::NotUtf8 { row, field } => write!(f, "row {row}: {} is not UTF-8", field.name()),
            Self::BadClient { row } => write!(f, "row {row}: bad client id"),
            Self::BadTx { row } => write!(f, "row {row}: bad tx id"),
            Self::BadAmount { row, reason } => write!(f, "row {row}: {reason}"),
            Self::ZeroAmount { row } => write!(f, "row {row}: amount is zero"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BadAmount { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    error: ParseError,
    burn: Option<TxId>,
}

impl Rejected {
    pub fn error(&self) -> &ParseError {
        &self.error
    }

    pub fn burn(&self) -> Option<TxId> {
        self.burn
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Event { row: u64, event: Event },
    Rejected(Rejected),
    MissingHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl Kind {
    fn moves_money(self) -> bool {
        matches!(self, Self::Deposit | Self::Withdrawal)
    }
}

pub struct Events<R> {
    records: ByteRecordsIntoIter<R>,
    row: u64,
    pending: Option<(u64, ByteRecord)>,
    started: bool,
    done: bool,
}

impl<R: Read> Iterator for Events<R> {
    type Item = Result<Row, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let item = self.next_item();
        if matches!(item, Some(Err(_))) {
            self.done = true;
        }
        item
    }
}

impl<R: Read> Events<R> {
    fn next_item(&mut self) -> Option<Result<Row, io::Error>> {
        if !self.started {
            self.started = true;
            match self.read_record()? {
                Err(error) => return Some(Err(error)),
                Ok((_, record)) if is_header(&record) => {}
                Ok(first) => {
                    self.pending = Some(first);
                    return Some(Ok(Row::MissingHeader));
                }
            }
        }
        let (row, record) = match self.pending.take() {
            Some(pending) => pending,
            None => match self.read_record()? {
                Ok(next) => next,
                Err(error) => return Some(Err(error)),
            },
        };
        Some(Ok(classify(row, &record)))
    }

    fn read_record(&mut self) -> Option<Result<(u64, ByteRecord), io::Error>> {
        match self.records.next()? {
            Ok(record) => {
                self.row = self.row.saturating_add(1);
                Some(Ok((self.row, record)))
            }
            Err(error) => Some(Err(to_io(error))),
        }
    }
}

pub fn read_events<R: Read>(reader: R) -> Events<R> {
    let records = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .quoting(false)
        .trim(Trim::All)
        .from_reader(reader)
        .into_byte_records();
    Events { records, row: 0, pending: None, started: false, done: false }
}

fn to_io(error: csv::Error) -> io::Error {
    match error.kind() {
        csv::ErrorKind::Io(source) => io::Error::new(source.kind(), error),
        _ => io::Error::other(error),
    }
}

fn has_extra_fields(record: &ByteRecord) -> bool {
    record.iter().skip(MAX_FIELDS).any(|field| !field.is_empty())
}

fn is_header(record: &ByteRecord) -> bool {
    record.len() >= HEADER.len()
        && !has_extra_fields(record)
        && HEADER.iter().zip(record.iter()).all(|(name, field)| {
            std::str::from_utf8(field).is_ok_and(|text| text.eq_ignore_ascii_case(name))
        })
}

fn classify(row: u64, record: &ByteRecord) -> Row {
    match parse_record(row, record) {
        Ok(event) => Row::Event { row, event },
        Err(rejected) => Row::Rejected(rejected),
    }
}

fn reject(error: ParseError, burn: Option<TxId>) -> Rejected {
    Rejected { error, burn }
}

fn parse_record(row: u64, record: &ByteRecord) -> Result<Event, Rejected> {
    let kind = parse_kind(row, record);
    let tx = parse_tx(row, record);
    let burn = match (&kind, &tx) {
        (Ok(kind), Ok(tx)) if kind.moves_money() => Some(*tx),
        _ => None,
    };
    if has_extra_fields(record) {
        return Err(reject(ParseError::TooManyFields { row }, burn));
    }
    let kind = kind.map_err(|error| reject(error, None))?;
    let client = parse_client(row, record);
    build(row, kind, record, client, tx).map_err(|error| reject(error, burn))
}

fn parse_kind(row: u64, record: &ByteRecord) -> Result<Kind, ParseError> {
    let bytes = record.get(0).unwrap_or_default();
    let text =
        std::str::from_utf8(bytes).map_err(|_| ParseError::NotUtf8 { row, field: Field::Type })?;
    KINDS
        .iter()
        .find(|(name, _)| text.eq_ignore_ascii_case(name))
        .map(|(_, kind)| *kind)
        .ok_or(ParseError::UnknownType { row })
}

fn field_text(
    row: u64,
    record: &ByteRecord,
    index: usize,
    field: Field,
) -> Result<&str, ParseError> {
    let bytes = record.get(index).ok_or(ParseError::MissingField { row, field })?;
    std::str::from_utf8(bytes).map_err(|_| ParseError::NotUtf8 { row, field })
}

fn parse_digits<T: FromStr>(text: &str) -> Option<T> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn parse_client(row: u64, record: &ByteRecord) -> Result<ClientId, ParseError> {
    let text = field_text(row, record, 1, Field::Client)?;
    parse_digits::<u16>(text).map(ClientId::new).ok_or(ParseError::BadClient { row })
}

fn parse_tx(row: u64, record: &ByteRecord) -> Result<TxId, ParseError> {
    let text = field_text(row, record, 2, Field::Tx)?;
    parse_digits::<u32>(text).map(TxId::new).ok_or(ParseError::BadTx { row })
}

fn parse_amount(row: u64, record: &ByteRecord) -> Result<Amount, ParseError> {
    let text = field_text(row, record, 3, Field::Amount)?;
    let amount = text.parse::<Amount>().map_err(|reason| ParseError::BadAmount { row, reason })?;
    if amount == Amount::ZERO {
        return Err(ParseError::ZeroAmount { row });
    }
    Ok(amount)
}

fn build(
    row: u64,
    kind: Kind,
    record: &ByteRecord,
    client: Result<ClientId, ParseError>,
    tx: Result<TxId, ParseError>,
) -> Result<Event, ParseError> {
    let client = client?;
    let tx = tx?;
    Ok(match kind {
        Kind::Deposit => Event::Deposit { client, tx, amount: parse_amount(row, record)? },
        Kind::Withdrawal => Event::Withdrawal { client, tx, amount: parse_amount(row, record)? },
        Kind::Dispute => Event::Dispute { client, tx },
        Kind::Resolve => Event::Resolve { client, tx },
        Kind::Chargeback => Event::Chargeback { client, tx },
    })
}

#[cfg(test)]
mod tests;
