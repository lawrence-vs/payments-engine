---
name: test-first-loop
description: Strict test-first implementation loop - approach memo and failing tests first (both need approval), then iterate autonomously against `./check.sh` until green, never touching files outside an agreed allowlist. Use when implementing a feature or fix where the approach is non-obvious and repeated back-and-forth redirection is likely.
---

# Test-first loop

Inverts the usual flow: the largest friction with an AI assistant is it committing to an
implementation before the approach is validated, forcing a mid-flight redirect. Here the agent self-corrects against failing tests
instead of against interruptions, because the contract is agreed up front.

Do not write implementation code until the user has approved both Step 1 and
Step 2.

## Step 1 — Approach memo (no code)

- Which files will be touched and why, as an explicit allowlist.
- Existing repo conventions/idioms found that this must follow — search
  first, cite examples (file:line).
- Two alternative approaches considered and why they were rejected.
- What could go wrong and what you're uncertain about.

Stop here and show the memo. Wait for approval.

## Step 2 — Test suite first

- Write the failing tests that define correct behaviour, including edge
  cases and at least one test that would catch a regression of the original
  bug (if this is a fix).
- Show the tests and confirm they fail for the right reason (not a compile
  error or unrelated failure).

Stop here and show the tests. Wait for approval.

## Step 3 — Autonomous loop (only after Steps 1 and 2 are approved)

- Implement, then iterate until `./check.sh` passes (fmt, clippy, tests, doc, audit);
  for a quick loop run `cargo test <name>` first.
- Never touch a file outside the approved allowlist. If it looks necessary,
  stop and ask first.
- Never commit plan files, scratch notes, or generated artefacts.
- If tempted to change a test to make it pass rather than fixing the code,
  stop and explain why instead of doing it silently.

## Step 4 — Report

- Diff summary, test results, any assumption made, anything a reviewer
  should look at closely.
- Stop before commit and push. Wait for explicit go-ahead; the engineer commits.
