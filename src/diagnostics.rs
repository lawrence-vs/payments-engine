//! Stderr reporting: the first 100 rejected rows in detail, then a count per category.

use std::fmt::{self, Display};
use std::io::Write;

use crate::EngineError;

const LOG_CAP: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    Bad,
    Failed,
    Ignored,
    NeedsReview,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) bad: u64,
    pub(crate) failed: u64,
    pub(crate) ignored: u64,
    pub(crate) needs_review: u64,
}

pub(crate) struct Diagnostics<W: Write> {
    sink: W,
    logged: usize,
    counts: Counts,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bad => "bad",
            Self::Failed => "failed",
            Self::Ignored => "ignored",
            Self::NeedsReview => "needs review",
        })
    }
}

impl<W: Write> Diagnostics<W> {
    pub(crate) fn new(sink: W) -> Self {
        Self { sink, logged: 0, counts: Counts::default() }
    }

    pub(crate) fn warn(&mut self, message: impl Display) {
        self.line(format_args!("warning: {message}"));
    }

    pub(crate) fn reject(&mut self, category: Category, error: impl Display) {
        let count = match category {
            Category::Bad => &mut self.counts.bad,
            Category::Failed => &mut self.counts.failed,
            Category::Ignored => &mut self.counts.ignored,
            Category::NeedsReview => &mut self.counts.needs_review,
        };
        *count = count.saturating_add(1);
        if self.logged < LOG_CAP {
            self.logged = self.logged.saturating_add(1);
            self.line(format_args!("{category}: {error}"));
        }
    }

    pub(crate) fn finish(mut self) {
        let counts = &self.counts;
        let summary = [
            (Category::Bad, counts.bad),
            (Category::Failed, counts.failed),
            (Category::Ignored, counts.ignored),
            (Category::NeedsReview, counts.needs_review),
        ];
        for (category, count) in summary {
            if count > 0 {
                self.line(format_args!("skipped {category}: {count}"));
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn counts(&self) -> &Counts {
        &self.counts
    }

    // Logging must never stop the run, so a failed write to the sink is dropped.
    fn line(&mut self, text: fmt::Arguments<'_>) {
        let _ = writeln!(self.sink, "{text}");
    }
}

pub(crate) fn category(error: &EngineError) -> Category {
    match error {
        EngineError::Locked { .. }
        | EngineError::InsufficientFunds { .. }
        | EngineError::Overflow { .. } => Category::Failed,
        EngineError::DuplicateTx { .. }
        | EngineError::UnknownTx { .. }
        | EngineError::NotDisputable { .. }
        | EngineError::NotUnderDispute { .. }
        | EngineError::ClientMismatch { .. } => Category::Ignored,
        EngineError::AlreadyDisputed { .. } => Category::NeedsReview,
        EngineError::NonPositive { .. } => Category::Failed,
    }
}

#[cfg(test)]
mod tests {
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
            ("non_positive", EngineError::NonPositive { tx }, Category::Failed),
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
}
