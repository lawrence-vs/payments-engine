use super::*;
use crate::{ClientId, EngineError, TxId};
use std::io;

struct AlwaysFails;

impl Write for AlwaysFails {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn copy_counts(counts: &Counts) -> Counts {
    Counts {
        bad: counts.bad,
        failed: counts.failed,
        ignored: counts.ignored,
        needs_review: counts.needs_review,
    }
}

fn sink_lines(sink: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(sink).lines().map(str::to_owned).collect()
}

#[test]
fn category_table() {
    let client = ClientId::new(1);
    let tx = TxId::new(1);
    let cases: &[(&str, EngineError, Category)] = &[
        ("locked", EngineError::Locked { client }, Category::Failed),
        ("duplicate_tx", EngineError::DuplicateTx { tx }, Category::Ignored),
        ("insufficient_funds", EngineError::InsufficientFunds { client, tx }, Category::Failed),
        ("overflow", EngineError::Overflow { client, tx }, Category::Failed),
        ("unknown_tx", EngineError::UnknownTx { tx }, Category::Ignored),
        ("not_disputable", EngineError::NotDisputable { tx }, Category::Ignored),
        ("not_under_dispute", EngineError::NotUnderDispute { tx }, Category::Ignored),
        ("client_mismatch", EngineError::ClientMismatch { client, tx }, Category::Ignored),
        ("already_disputed", EngineError::AlreadyDisputed { tx }, Category::NeedsReview),
    ];
    let mut failures = Vec::new();
    for (name, error, want) in cases {
        let got = category(error);
        if got != *want {
            failures.push(format!("{name}: got {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn reject_cap_table() {
    let cases: &[(&str, u64, usize)] =
        &[("three_rejects", 3, 3), ("exactly_cap", 100, 100), ("one_over_cap", 101, 100)];
    let mut failures = Vec::new();
    for (name, rejects, want_lines) in cases {
        let mut sink: Vec<u8> = Vec::new();
        let got_counts = {
            let mut diagnostics = Diagnostics::new(&mut sink);
            for _ in 0..*rejects {
                diagnostics.reject(Category::Bad, "e");
            }
            copy_counts(diagnostics.counts())
        };
        let want_counts = Counts { bad: *rejects, ..Counts::default() };
        if got_counts != want_counts {
            failures.push(format!("{name}: counts {got_counts:?}, want {want_counts:?}"));
        }
        let got_lines = sink_lines(&sink).len();
        if got_lines != *want_lines {
            failures.push(format!("{name}: {got_lines} lines written, want {want_lines}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn skipped_summary_after_cap() {
    let mut sink: Vec<u8> = Vec::new();
    let got_counts = {
        let mut diagnostics = Diagnostics::new(&mut sink);
        for _ in 0..250 {
            diagnostics.reject(Category::Bad, "e");
        }
        diagnostics.reject(Category::NeedsReview, "e");
        let counts = copy_counts(diagnostics.counts());
        diagnostics.finish();
        counts
    };
    let mut failures = Vec::new();
    let want_counts = Counts { bad: 250, needs_review: 1, ..Counts::default() };
    if got_counts != want_counts {
        failures.push(format!("counts {got_counts:?}, want {want_counts:?}"));
    }
    let lines = sink_lines(&sink);
    if lines.len() != 102 {
        failures.push(format!("{} lines written, want 102", lines.len()));
    }
    let want_tail = ["skipped bad: 250".to_owned(), "skipped needs review: 1".to_owned()];
    if !lines.ends_with(&want_tail) {
        failures.push(format!("last lines {:?}, want {want_tail:?}", lines.last()));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn finish_without_rejects_writes_nothing() {
    let mut sink: Vec<u8> = Vec::new();
    Diagnostics::new(&mut sink).finish();
    assert!(sink.is_empty(), "wrote {:?}", sink_lines(&sink));
}

#[test]
fn warn_after_cap_is_written_and_not_counted() {
    let mut sink: Vec<u8> = Vec::new();
    let got_counts = {
        let mut diagnostics = Diagnostics::new(&mut sink);
        for _ in 0..100 {
            diagnostics.reject(Category::Bad, "e");
        }
        diagnostics.warn("w");
        copy_counts(diagnostics.counts())
    };
    let mut failures = Vec::new();
    let want_counts = Counts { bad: 100, ..Counts::default() };
    if got_counts != want_counts {
        failures.push(format!("counts {got_counts:?}, want {want_counts:?}"));
    }
    let lines = sink_lines(&sink);
    if lines.len() != 101 || lines.last().map(String::as_str) != Some("warning: w") {
        failures.push(format!("lines {lines:?}, want 101 lines ending in warning: w"));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn failing_sink_does_not_panic() {
    let mut diagnostics = Diagnostics::new(AlwaysFails);
    diagnostics.warn("w");
    diagnostics.reject(Category::Bad, "e");
    let got = copy_counts(diagnostics.counts());
    diagnostics.finish();
    let want = Counts { bad: 1, ..Counts::default() };
    assert!(got == want, "counts {got:?}, want {want:?}");
}
