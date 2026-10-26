use super::*;
use crate::{Amount, ClientId};
use std::io::{self, Read};

struct FailsAfter(&'static [u8]);

impl Read for FailsAfter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.0.is_empty() {
            return Err(io::Error::other("read failed"));
        }
        let n = self.0.len().min(buf.len());
        let (head, tail) = self.0.split_at(n);
        let (out, _) = buf.split_at_mut(n);
        out.copy_from_slice(head);
        self.0 = tail;
        Ok(n)
    }
}

fn record<T: AsRef<[u8]>>(fields: &[T]) -> csv::ByteRecord {
    let mut record = csv::ByteRecord::new();
    for field in fields {
        record.push_field(field.as_ref());
    }
    record
}

fn split(result: Result<Event, Rejected>) -> (Result<Event, ParseError>, Option<u32>) {
    match result {
        Ok(event) => (Ok(event), None),
        Err(rejected) => (Err(rejected.error().clone()), rejected.burn().map(TxId::get)),
    }
}

fn deposit(client: u16, tx: u32, units: i64) -> Event {
    Event::Deposit {
        client: ClientId::new(client),
        tx: TxId::new(tx),
        amount: Amount::from_units(units),
    }
}

fn withdrawal(client: u16, tx: u32, units: i64) -> Event {
    Event::Withdrawal {
        client: ClientId::new(client),
        tx: TxId::new(tx),
        amount: Amount::from_units(units),
    }
}

fn dispute(client: u16, tx: u32) -> Event {
    Event::Dispute { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn resolve(client: u16, tx: u32) -> Event {
    Event::Resolve { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn chargeback(client: u16, tx: u32) -> Event {
    Event::Chargeback { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn event_text(event: &Event) -> String {
    match event {
        Event::Deposit { client, tx, amount } => {
            format!("D({},{},{})", client.get(), tx.get(), amount.units())
        }
        Event::Withdrawal { client, tx, amount } => {
            format!("W({},{},{})", client.get(), tx.get(), amount.units())
        }
        Event::Dispute { client, tx } => format!("Dispute({},{})", client.get(), tx.get()),
        Event::Resolve { client, tx } => format!("Resolve({},{})", client.get(), tx.get()),
        Event::Chargeback { client, tx } => format!("Chargeback({},{})", client.get(), tx.get()),
    }
}

fn variant_name(error: &ParseError) -> (u64, &'static str) {
    match error {
        ParseError::UnknownType { row } => (*row, "UnknownType"),
        ParseError::MissingField { row, .. } => (*row, "MissingField"),
        ParseError::TooManyFields { row } => (*row, "TooManyFields"),
        ParseError::NotUtf8 { row, .. } => (*row, "NotUtf8"),
        ParseError::BadClient { row } => (*row, "BadClient"),
        ParseError::BadTx { row } => (*row, "BadTx"),
        ParseError::BadAmount { row, .. } => (*row, "BadAmount"),
        ParseError::ZeroAmount { row } => (*row, "ZeroAmount"),
    }
}

fn token(item: Result<Row, io::Error>) -> String {
    match item {
        Err(_) => "IO".to_string(),
        Ok(Row::MissingHeader) => "H".to_string(),
        Ok(Row::Event(event)) => format!("E {}", event_text(&event)),
        Ok(Row::Rejected(rejected)) => {
            let (row, name) = variant_name(rejected.error());
            format!("R {row} {name}")
        }
    }
}

#[test]
fn parse_record_table() {
    let cases: &[(&str, &[&str], Result<Event, ParseError>)] = &[
        ("ok_deposit", &["deposit", "1", "1", "1.0"], Ok(deposit(1, 1, 10000))),
        ("ok_withdrawal", &["withdrawal", "2", "5", "3.0"], Ok(withdrawal(2, 5, 30000))),
        ("ok_dispute3", &["dispute", "1", "1"], Ok(dispute(1, 1))),
        ("ok_dispute4_empty", &["dispute", "1", "1", ""], Ok(dispute(1, 1))),
        ("ok_dispute4_junk", &["dispute", "1", "1", "abc"], Ok(dispute(1, 1))),
        ("ok_resolve", &["resolve", "1", "1"], Ok(resolve(1, 1))),
        ("ok_chargeback", &["chargeback", "1", "1"], Ok(chargeback(1, 1))),
        ("case_Deposit", &["Deposit", "1", "1", "1.0"], Ok(deposit(1, 1, 10000))),
        ("case_DEPOSIT", &["DEPOSIT", "1", "1", "1.0"], Ok(deposit(1, 1, 10000))),
        ("case_dEpOsIt", &["dEpOsIt", "1", "1", "1.0"], Ok(deposit(1, 1, 10000))),
        ("max_client", &["deposit", "65535", "1", "1"], Ok(deposit(65535, 1, 10000))),
        ("max_tx", &["deposit", "1", "4294967295", "1"], Ok(deposit(1, 4294967295, 10000))),
        ("lenient_amount_dot", &["deposit", "1", "1", ".5"], Ok(deposit(1, 1, 5000))),
        ("lenient_amount_trailing_dot", &["deposit", "1", "1", "1."], Ok(deposit(1, 1, 10000))),
        ("rej_unknown_type", &["deposits", "1", "1", "1"], Err(ParseError::UnknownType { row: 1 })),
        ("rej_empty_type", &["", "1", "1", "1"], Err(ParseError::UnknownType { row: 1 })),
        (
            "rej_client_65536",
            &["deposit", "65536", "1", "1"],
            Err(ParseError::BadClient { row: 1 }),
        ),
        ("rej_client_neg", &["deposit", "-1", "1", "1"], Err(ParseError::BadClient { row: 1 })),
        ("rej_client_plus", &["deposit", "+1", "1", "1"], Err(ParseError::BadClient { row: 1 })),
        ("rej_client_alpha", &["deposit", "abc", "1", "1"], Err(ParseError::BadClient { row: 1 })),
        ("rej_client_empty", &["deposit", "", "1", "1"], Err(ParseError::BadClient { row: 1 })),
        (
            "rej_client_decimal",
            &["deposit", "1.0", "1", "1"],
            Err(ParseError::BadClient { row: 1 }),
        ),
        ("rej_tx_big", &["deposit", "1", "4294967296", "1"], Err(ParseError::BadTx { row: 1 })),
        ("rej_tx_neg", &["deposit", "1", "-1", "1"], Err(ParseError::BadTx { row: 1 })),
        ("rej_tx_plus", &["deposit", "1", "+1", "1"], Err(ParseError::BadTx { row: 1 })),
        (
            "rej_amount_empty",
            &["deposit", "1", "1", ""],
            Err(ParseError::BadAmount { row: 1, reason: AmountError::Empty }),
        ),
        (
            "rej_amount_neg",
            &["deposit", "1", "1", "-5"],
            Err(ParseError::BadAmount { row: 1, reason: AmountError::Malformed }),
        ),
        (
            "rej_amount_exp",
            &["deposit", "1", "1", "1e3"],
            Err(ParseError::BadAmount { row: 1, reason: AmountError::Malformed }),
        ),
        (
            "rej_amount_5dec",
            &["deposit", "1", "1", "0.00001"],
            Err(ParseError::BadAmount { row: 1, reason: AmountError::TooManyDecimals }),
        ),
        (
            "rej_amount_overflow",
            &["deposit", "1", "1", "922337203685477.5808"],
            Err(ParseError::BadAmount { row: 1, reason: AmountError::Overflow }),
        ),
        ("rej_zero", &["deposit", "1", "1", "0"], Err(ParseError::ZeroAmount { row: 1 })),
        (
            "rej_zero_padded",
            &["withdrawal", "1", "1", "0.0000"],
            Err(ParseError::ZeroAmount { row: 1 }),
        ),
        (
            "rej_missing_amount",
            &["deposit", "1", "1"],
            Err(ParseError::MissingField { row: 1, field: Field::Amount }),
        ),
        (
            "rej_missing_tx",
            &["deposit", "1"],
            Err(ParseError::MissingField { row: 1, field: Field::Tx }),
        ),
        (
            "rej_dispute_missing_tx",
            &["dispute", "1"],
            Err(ParseError::MissingField { row: 1, field: Field::Tx }),
        ),
        (
            "rej_five_fields",
            &["deposit", "1", "1", "1", "x"],
            Err(ParseError::TooManyFields { row: 1 }),
        ),
        (
            "rej_first_error_wins",
            &["deposit", "x", "y", "z"],
            Err(ParseError::BadClient { row: 1 }),
        ),
    ];

    let mut failures = Vec::new();
    for (name, fields, expected) in cases {
        let (got, _) = split(parse_record(1, &record(fields)));
        if got != *expected {
            failures.push(format!("{name}: fields {fields:?}: got {got:?}, want {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn burn_table() {
    let cases: &[(&str, &[&str], Option<u32>)] = &[
        ("burn_bad_amount", &["deposit", "1", "5", "abc"], Some(5)),
        ("burn_zero", &["withdrawal", "1", "6", "0"], Some(6)),
        ("burn_bad_client_good_tx", &["deposit", "abc", "7", "1"], Some(7)),
        ("burn_missing_amount", &["deposit", "1", "8"], Some(8)),
        ("burn_five_fields", &["deposit", "1", "9", "1", "x"], Some(9)),
        ("burn_bad_tx", &["deposit", "1", "x", "1"], None),
        ("burn_missing_tx", &["deposit", "1"], None),
        ("burn_unknown_type", &["foo", "1", "10", "1"], None),
        ("burn_dispute_bad_client", &["dispute", "x", "10"], None),
        ("burn_resolve_missing_tx", &["resolve", "1"], None),
    ];

    let mut failures = Vec::new();
    for (name, fields, expected) in cases {
        let (_, got) = split(parse_record(1, &record(fields)));
        if got != *expected {
            failures.push(format!("{name}: fields {fields:?}: burn {got:?}, want {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn not_utf8_table() {
    type Case = (&'static str, &'static [&'static [u8]], Result<Event, ParseError>, Option<u32>);
    let cases: &[Case] = &[
        (
            "amount_bytes",
            &[b"deposit", b"1", b"1", b"\xFF"],
            Err(ParseError::NotUtf8 { row: 1, field: Field::Amount }),
            Some(1),
        ),
        (
            "type_bytes",
            &[b"\xFF", b"1", b"1", b"1"],
            Err(ParseError::NotUtf8 { row: 1, field: Field::Type }),
            None,
        ),
        (
            "client_bytes",
            &[b"deposit", b"\xFF", b"1", b"1"],
            Err(ParseError::NotUtf8 { row: 1, field: Field::Client }),
            Some(1),
        ),
    ];

    let mut failures = Vec::new();
    for (name, fields, expected_event, expected_burn) in cases {
        let (got_event, got_burn) = split(parse_record(1, &record(fields)));
        if got_event != *expected_event || got_burn != *expected_burn {
            failures.push(format!(
                "{name}: fields {fields:?}: got {got_event:?} burn {got_burn:?}, want {expected_event:?} burn {expected_burn:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn read_events_table() {
    let cases: &[(&str, &[u8], &[&str])] = &[
        ("header_plain", b"type,client,tx,amount\ndeposit,1,1,1.0\n", &["E D(1,1,10000)"]),
        (
            "header_upper_spaces",
            b" TYPE , Client,tx , Amount\ndeposit,1,1,1.0\n",
            &["E D(1,1,10000)"],
        ),
        (
            "header_bom",
            b"\xEF\xBB\xBFtype,client,tx,amount\ndeposit,1,1,1.0\n",
            &["E D(1,1,10000)"],
        ),
        ("crlf", b"type,client,tx,amount\r\ndeposit,1,1,1.0\r\n", &["E D(1,1,10000)"]),
        (
            "no_header",
            b"deposit,1,1,10\ndeposit,1,2,5\n",
            &["H", "E D(1,1,100000)", "E D(1,2,50000)"],
        ),
        ("empty_input", b"", &[]),
        ("header_only_no_newline", b"type,client,tx,amount", &[]),
        (
            "blank_lines_skipped",
            b"type,client,tx,amount\n\ndeposit,1,1,1.0\n\n\ndeposit,1,2,2.0\n",
            &["E D(1,1,10000)", "E D(1,2,20000)"],
        ),
        (
            "short_rows_mixed",
            b"type,client,tx,amount\ndispute,1,1\nresolve,1,1,\n",
            &["E Dispute(1,1)", "E Resolve(1,1)"],
        ),
        (
            "tabs_around_fields",
            b"type,client,tx,amount\n\tdeposit\t,\t1\t,1,\t1.0\t\n",
            &["E D(1,1,10000)"],
        ),
        (
            "quoted_fields",
            b"type,client,tx,amount\n\"deposit\",\"1\",1,\"1.0\"\n",
            &["E D(1,1,10000)"],
        ),
        (
            "header_mid_file",
            b"type,client,tx,amount\ndeposit,1,1,1.0\ntype,client,tx,amount\n",
            &["E D(1,1,10000)", "R 3 UnknownType"],
        ),
        (
            "row_numbers",
            b"type,client,tx,amount\ndeposit,1,1,1.0\ndeposit,1,2,1.0\ndeposit,x,3,1.0\n",
            &["E D(1,1,10000)", "E D(1,2,10000)", "R 4 BadClient"],
        ),
        ("bom_without_header", b"\xEF\xBB\xBFdeposit,1,1,1\n", &["H", "E D(1,1,10000)"]),
        (
            "non_utf8_field_in_file",
            b"type,client,tx,amount\ndeposit,1,1,1.0\ndeposit,1,2,\xFF\ndeposit,1,3,1.0\n",
            &["E D(1,1,10000)", "R 3 NotUtf8", "E D(1,3,10000)"],
        ),
    ];

    let mut failures = Vec::new();
    for (name, input, expected) in cases {
        let got: Vec<String> = read_events(*input).map(token).collect();
        let want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        if got != want {
            failures.push(format!("{name}: got {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn io_error_table() {
    let cases: &[(&str, &'static [u8], &[&str])] = &[
        ("first_read_fails", b"", &["IO"]),
        (
            "fails_after_valid_row",
            b"type,client,tx,amount\ndeposit,1,1,1.0\n",
            &["E D(1,1,10000)", "IO"],
        ),
    ];

    let mut failures = Vec::new();
    for (name, data, expected) in cases {
        let got: Vec<String> = read_events(FailsAfter(data)).map(token).collect();
        let want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        if got != want {
            failures.push(format!("{name}: got {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
