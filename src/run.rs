//! The whole pipeline: read rows, apply events, log rejected rows, write accounts.

use std::io::{self, Read, Write};

use crate::diagnostics::{Category, Diagnostics, category};
use crate::output::write_accounts;
use crate::{Engine, Row, read_events};

pub fn run<R: Read, W: Write, E: Write>(input: R, mut out: W, err: E) -> io::Result<()> {
    let mut engine = Engine::new();
    let mut diagnostics = Diagnostics::new(err);
    for item in read_events(input) {
        match item? {
            Row::MissingHeader => {
                diagnostics.warn("first line is not a header, read as data");
            }
            Row::Rejected(rejected) => {
                diagnostics.reject(Category::Bad, rejected.error());
                if let Some(tx) = rejected.burn() {
                    engine.burn(tx);
                }
            }
            Row::Event { row, event } => {
                if let Err(error) = engine.apply(event) {
                    diagnostics.reject(category(&error), format_args!("row {row}: {error}"));
                }
            }
        }
    }
    write_accounts(&mut out, &engine)?;
    out.flush()?;
    diagnostics.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    struct AlwaysFails;

    impl Write for AlwaysFails {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn run_table() {
        let cases: &[(&str, &str, &str, &str)] = &[
            (
                "clean_run_has_empty_stderr",
                "type, client, tx, amount\ndeposit,1,1,1.0\ndeposit,2,2,2.0\n",
                "client,available,held,total,locked\n1,1.0000,0.0000,1.0000,false\n2,2.0000,0.0000,2.0000,false\n",
                "",
            ),
            (
                "missing_header_warns_and_keeps_row",
                "deposit,1,1,10\n",
                "client,available,held,total,locked\n1,10.0000,0.0000,10.0000,false\n",
                "warning: first line is not a header, read as data\n",
            ),
            (
                "engine_refusals_name_their_row",
                "type,client,tx,amount\ndeposit,1,1,10\nwithdrawal,1,2,50\ndispute,1,9\n",
                "client,available,held,total,locked\n1,10.0000,0.0000,10.0000,false\n",
                "failed: row 3: client 1 has insufficient funds for transaction 2\nignored: row 4: transaction 9 is unknown\nskipped failed: 1\nskipped ignored: 1\n",
            ),
            (
                "parse_failure_burns_id_end_to_end",
                "type,client,tx,amount\ndeposit,1,5,abc\ndeposit,2,5,7\n",
                "client,available,held,total,locked\n",
                "bad: row 2: amount is malformed\nignored: row 3: transaction 5 already exists\nskipped bad: 1\nskipped ignored: 1\n",
            ),
        ];
        let mut failures = Vec::new();
        for (name, input, want_out, want_err) in cases {
            let mut out: Vec<u8> = Vec::new();
            let mut err: Vec<u8> = Vec::new();
            let result = run(input.as_bytes(), &mut out, &mut err);
            let got_out = String::from_utf8_lossy(&out).into_owned();
            let got_err = String::from_utf8_lossy(&err).into_owned();
            if result.is_err() || got_out != *want_out || got_err != *want_err {
                failures.push(format!(
                    "{name}: result {result:?}, out {got_out:?}, err {got_err:?}; want out {want_out:?}, err {want_err:?}"
                ));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn read_failure_writes_nothing() {
        let input = FailsAfter(b"type,client,tx,amount\ndeposit,1,1,1.0\n");
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let result = run(input, &mut out, &mut err);
        assert!(result.is_err(), "read failure was not returned: {result:?}");
        assert!(out.is_empty(), "stdout written before the read failed: {out:?}");
    }

    #[test]
    fn bad_rows_are_capped() {
        let mut input = String::from("type,client,tx,amount\n");
        for _ in 0..150 {
            input.push_str("foo,1,1,1\n");
        }
        input.push_str("deposit,1,1,10\ndispute,1,1\nresolve,1,1\ndispute,1,1\n");
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let result = run(input.as_bytes(), &mut out, &mut err);
        let want_out = "client,available,held,total,locked\n1,10.0000,0.0000,10.0000,false\n";
        let want_err: String = (2..=101)
            .map(|row| format!("bad: row {row}: unknown event type\n"))
            .chain(
                ["skipped bad: 150\n", "skipped needs review: 1\n"].into_iter().map(String::from),
            )
            .collect();
        let got_out = String::from_utf8_lossy(&out).into_owned();
        let got_err = String::from_utf8_lossy(&err).into_owned();
        let mut failures = Vec::new();
        if result.is_err() || got_out != want_out || got_err != want_err {
            failures.push(format!(
                "result {result:?}, out {got_out:?}, err {got_err:?}; want out {want_out:?}, err {want_err:?}"
            ));
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn stdout_failure_is_returned() {
        let mut err: Vec<u8> = Vec::new();
        let input = "type,client,tx,amount\ndeposit,1,1,1.0\n";
        let result = run(input.as_bytes(), AlwaysFails, &mut err);
        assert!(result.is_err(), "stdout write error was not returned: {result:?}");
    }
}
