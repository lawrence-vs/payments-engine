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
        f.write_str("")
    }
}

impl<W: Write> Diagnostics<W> {
    pub(crate) fn new(sink: W) -> Self {
        Self { sink, logged: 0, counts: Counts::default() }
    }

    pub(crate) fn warn(&mut self, _message: impl Display) {}

    pub(crate) fn reject(&mut self, _category: Category, _error: impl Display) {}

    pub(crate) fn finish(self) {}

    pub(crate) fn counts(&self) -> &Counts {
        &self.counts
    }
}

pub(crate) fn category(_error: &EngineError) -> Category {
    Category::Bad
}

#[cfg(test)]
mod tests;
