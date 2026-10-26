use super::*;
use crate::{Amount, ClientId, Event, TxId};

fn dep(client: u16, tx: u32, units: i64) -> Event {
    Event::Deposit {
        client: ClientId::new(client),
        tx: TxId::new(tx),
        amount: Amount::from_units(units),
    }
}

fn wd(client: u16, tx: u32, units: i64) -> Event {
    Event::Withdrawal {
        client: ClientId::new(client),
        tx: TxId::new(tx),
        amount: Amount::from_units(units),
    }
}

fn disp(client: u16, tx: u32) -> Event {
    Event::Dispute { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn cb(client: u16, tx: u32) -> Event {
    Event::Chargeback { client: ClientId::new(client), tx: TxId::new(tx) }
}

struct AlwaysFails;

impl Write for AlwaysFails {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn render(events: &[Event]) -> String {
    let mut engine = Engine::new();
    for event in events {
        let _ = engine.apply(*event);
    }
    let mut out: Vec<u8> = Vec::new();
    let _ = write_accounts(&mut out, &engine);
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn write_accounts_table() {
    let cases: &[(&str, &[Event], &str)] = &[
        ("empty_engine", &[], "client,available,held,total,locked\n"),
        (
            "one_client",
            &[dep(1, 1, 150000)],
            "client,available,held,total,locked\n1,15.0000,0.0000,15.0000,false\n",
        ),
        (
            "numeric_order",
            &[dep(10, 1, 10000), dep(2, 2, 20000), dep(1, 3, 30000)],
            "client,available,held,total,locked\n\
             1,3.0000,0.0000,3.0000,false\n\
             2,2.0000,0.0000,2.0000,false\n\
             10,1.0000,0.0000,1.0000,false\n",
        ),
        (
            "negative_available",
            &[dep(1, 1, 100000), wd(1, 2, 80000), disp(1, 1)],
            "client,available,held,total,locked\n1,-8.0000,10.0000,2.0000,false\n",
        ),
        (
            "locked_flag",
            &[dep(1, 1, 50000), disp(1, 1), cb(1, 1)],
            "client,available,held,total,locked\n1,0.0000,0.0000,0.0000,true\n",
        ),
        (
            "padding_small",
            &[dep(1, 1, 1)],
            "client,available,held,total,locked\n1,0.0001,0.0000,0.0001,false\n",
        ),
        (
            "padding_negative_fraction",
            &[dep(1, 1, 500), dep(1, 2, 100), disp(1, 2), cb(1, 2)],
            "client,available,held,total,locked\n1,0.0500,0.0000,0.0500,true\n",
        ),
        (
            "negative_padding",
            &[
                dep(1, 1, 100),
                wd(1, 2, 100),
                dep(1, 3, 500),
                wd(1, 4, 500),
                dep(1, 5, 1),
                disp(1, 3),
            ],
            "client,available,held,total,locked\n1,-0.0499,0.0500,0.0001,false\n",
        ),
        (
            "extremes",
            &[dep(1, 1, i64::MAX)],
            "client,available,held,total,locked\n1,922337203685477.5807,0.0000,922337203685477.5807,false\n",
        ),
    ];
    let mut failures = Vec::new();
    for (name, events, want) in cases {
        let got = render(events);
        if got != *want {
            failures.push(format!("{name}: got {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn write_error_propagates() {
    let mut engine = Engine::new();
    let _ = engine.apply(dep(1, 1, 1));
    let result = write_accounts(&mut AlwaysFails, &engine);
    assert!(result.is_err(), "write error was not returned: {result:?}");
}

type PlantedCase<'a> = (&'a str, &'a [(u16, i64, i64)]);

#[test]
fn total_overflow_is_an_error_and_writes_nothing() {
    let cases: &[PlantedCase<'_>] = &[
        ("broken_only", &[(1, i64::MAX, 1)]),
        ("healthy_then_broken", &[(1, 10, 0), (2, i64::MAX, 1)]),
        ("broken_then_healthy", &[(1, i64::MAX, 1), (2, 10, 0)]),
    ];

    let mut failures = Vec::new();
    for (name, accounts) in cases {
        let mut engine = Engine::new();
        for (client, available, held) in *accounts {
            engine.plant_account(
                ClientId::new(*client),
                Amount::from_units(*available),
                Amount::from_units(*held),
            );
        }
        let mut out: Vec<u8> = Vec::new();
        let result = write_accounts(&mut out, &engine);
        let kind = result.as_ref().err().map(io::Error::kind);
        if kind != Some(io::ErrorKind::InvalidData) || !out.is_empty() {
            failures.push(format!("{name}: error kind {kind:?}, wrote {} bytes", out.len()));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
