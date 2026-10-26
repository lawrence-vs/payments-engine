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
mod tests;
