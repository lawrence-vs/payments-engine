use std::ffi::OsStr;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;
use std::str::FromStr;

const USAGE: &str = "usage: gen <rows> [seed] [clients]";
const DEFAULT_SEED: u64 = 1;
const DEFAULT_CLIENTS: u16 = 1000;
const DEPOSIT_RING: usize = 256;
const DISPUTED_RING: usize = 64;

struct Args {
    rows: u64,
    seed: u64,
    clients: u16,
}

struct Rng {
    state: u64,
}

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    // n is never zero at the call sites; checked_rem keeps the arithmetic lint quiet.
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64().checked_rem(n).unwrap_or(0)
    }
}

struct Ring<const N: usize> {
    slots: [(u16, u32); N],
    filled: usize,
    next: usize,
}

impl<const N: usize> Ring<N> {
    fn new() -> Self {
        Self { slots: [(0, 0); N], filled: 0, next: 0 }
    }

    fn push(&mut self, item: (u16, u32)) {
        if let Some(slot) = self.slots.get_mut(self.next) {
            *slot = item;
        }
        self.next = self.next.wrapping_add(1).checked_rem(N).unwrap_or(0);
        self.filled = self.filled.saturating_add(1).min(N);
    }

    fn pick(&self, rng: &mut Rng) -> Option<(u16, u32)> {
        if self.filled == 0 {
            return None;
        }
        let index = rng.below(u64::try_from(self.filled).ok()?);
        self.slots.get(usize::try_from(index).ok()?).copied()
    }
}

struct Stream {
    rng: Rng,
    clients: u16,
    next_tx: u32,
    deposits: Ring<DEPOSIT_RING>,
    disputed: Ring<DISPUTED_RING>,
}

impl Stream {
    fn new(args: &Args) -> Self {
        Self {
            rng: Rng { state: args.seed },
            clients: args.clients,
            next_tx: 1,
            deposits: Ring::new(),
            disputed: Ring::new(),
        }
    }

    fn row<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let pick = self.rng.next_u64() % 10_000;
        match pick {
            0..=5499 => self.deposit(out),
            5500..=7999 => self.withdrawal(out),
            8000..=8999 => match self.deposits.pick(&mut self.rng) {
                Some((client, tx)) => {
                    self.disputed.push((client, tx));
                    self.reference(out, "dispute", client, tx)
                }
                None => self.deposit(out),
            },
            9000..=9503 => {
                let kind = if pick < 9500 { "resolve" } else { "chargeback" };
                match self.disputed.pick(&mut self.rng) {
                    Some((client, tx)) => self.reference(out, kind, client, tx),
                    None => self.deposit(out),
                }
            }
            9504..=9506 => {
                let client = self.client();
                let tx = self.take_tx();
                match pick {
                    9504 => writeln!(out, "deposit,{client},{tx},0"),
                    9505 => writeln!(out, "foo,{client},{tx},1"),
                    _ => writeln!(out, "deposit,{client},{tx},abc"),
                }
            }
            9507..=9519 => match self.deposits.pick(&mut self.rng) {
                Some((_, old_tx)) => {
                    let client = self.client();
                    writeln!(out, "deposit,{client},{old_tx},5.0000")
                }
                None => self.deposit(out),
            },
            _ => self.deposit(out),
        }
    }

    fn deposit<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let client = self.client();
        let tx = self.take_tx();
        self.deposits.push((client, tx));
        let (whole, frac) = self.amount();
        writeln!(out, "deposit,{client},{tx},{whole}.{frac:04}")
    }

    fn withdrawal<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let client = self.client();
        let tx = self.take_tx();
        let (whole, frac) = self.amount();
        writeln!(out, "withdrawal,{client},{tx},{whole}.{frac:04}")
    }

    fn reference<W: Write>(
        &mut self,
        out: &mut W,
        kind: &str,
        client: u16,
        tx: u32,
    ) -> io::Result<()> {
        if self.rng.next_u64().is_multiple_of(2) {
            writeln!(out, "{kind},{client},{tx}")
        } else {
            writeln!(out, "{kind},{client},{tx},")
        }
    }

    fn client(&mut self) -> u16 {
        let index = self.rng.below(u64::from(self.clients));
        u16::try_from(index.wrapping_add(1)).unwrap_or(self.clients)
    }

    fn take_tx(&mut self) -> u32 {
        let tx = self.next_tx;
        self.next_tx = self.next_tx.saturating_add(1);
        tx
    }

    fn amount(&mut self) -> (u64, u64) {
        let units = self.rng.below(1_000_000).wrapping_add(1);
        (units / 10_000, units % 10_000)
    }
}

fn parse_arg<T: FromStr>(arg: &OsStr) -> Option<T> {
    arg.to_str()?.parse().ok()
}

fn parse_args() -> Option<Args> {
    let mut it = std::env::args_os().skip(1);
    let rows = parse_arg(&it.next()?)?;
    let seed = match it.next() {
        Some(arg) => parse_arg(&arg)?,
        None => DEFAULT_SEED,
    };
    let clients = match it.next() {
        Some(arg) => parse_arg(&arg)?,
        None => DEFAULT_CLIENTS,
    };
    if it.next().is_some() || clients == 0 {
        return None;
    }
    Some(Args { rows, seed, clients })
}

fn write_rows(args: &Args) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    writeln!(out, "type,client,tx,amount")?;
    let mut stream = Stream::new(args);
    for _ in 0..args.rows {
        stream.row(&mut out)?;
    }
    out.flush()
}

fn main() -> ExitCode {
    let Some(args) = parse_args() else {
        let _ = writeln!(io::stderr().lock(), "{USAGE}");
        return ExitCode::from(2);
    };
    match write_rows(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = writeln!(io::stderr().lock(), "write error: {e}");
            ExitCode::from(1)
        }
    }
}
