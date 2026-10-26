---
name: start-workflow
description: Drives a timed take-home from brief to submission as a Plan -> Red -> Act -> Verify loop - the engineer decides, Sonnet plans and reviews, a Haiku subagent builds one slice at a time from a blueprint, every claim is run before it is trusted. Use when starting the assignment, a new ticket, or the next slice.
---

# start-workflow

Reply terse in chat: fragments, tables, no filler. Docs written for reviewers (README, DESIGN, DECISIONS, AI_USAGE) use normal prose in the engineer's words.

The submission is the code and the trail. The engineer decides and can explain every line. Code standards: `rust-review` skill. Repo rules: `CLAUDE.md`.

## Roles
| Who | Does | Never |
|---|---|---|
| Engineer | picks shapes and policies, approves plans and red tests, reads every diff, commits, writes README and AI_USAGE | accepts a diff they cannot explain |
| Sonnet (main) | options with evidence, blueprints, review (`./check.sh` + `rust-review`), one question per slice | picks a policy silently, commits |
| Haiku (subagent) | builds one slice from a blueprint; crate versions and docs.rs lookups | changes the design, touches files outside the allowlist, commits |
| Opus (subagent) | one attack on DESIGN.md, one on the test plan, final review | fixes what it finds |
Stuck = two failed attempts or 15 minutes: a fresh Opus subagent gets only the failing test and the code, returns a ranked diagnosis; the engineer decides.

## Phase 0: clean session (before the first prompt)
1. `/mcp` lists nothing; `/context` shows no MCP tools and no skills beyond `.claude/skills/`. Anything listed -> disable it in `/mcp` first.
2. `git status` clean; `./check.sh` green on the skeleton.

## Phase 1: plan (plan mode, no edits)
1. Quote the brief text for every rule before framing options; never decide from a summary.
2. Shape: 2-3 options with trade-offs; the engineer picks.
3. Policies one at a time (AskUserQuestion), evidence first: 2-3 options incl. the simplest, the test that separates them, the command and its output. Row in `DECISIONS.md`.
4. `DESIGN.md`: types, rules table, assumptions, errors, test plan. `PLAN.md`: tickets `t-NN-<slug>` in build order, each with acceptance tests and a diff budget for code and for tests separately.
5. Opus attack on DESIGN and the test plan: "which bug still passes these tests?" Every finding answered in `DECISIONS.md`.

## The loop, one slice at a time (branch per ticket)
| Step | Who | Output | Gate |
|---|---|---|---|
| Plan | Sonnet | `test-first-loop` memo: allowlist, conventions with file:line, 2 rejected options, risks | engineer approves |
| Red | Haiku from blueprint, Sonnet checks | table tests, stub signatures so the failure is a wrong value, not a missing symbol | red output shown, fails for the right reason |
| Act | Haiku from blueprint | smallest code that turns the table green | Haiku reports: files changed, `./check.sh` result, anything it was unsure of |
| Verify | Sonnet | re-run `./check.sh` itself (never relay Haiku's claim), `rust-review` on `git diff`, every item reported | all pass; engineer reads the diff, answers one question |
| Commit | engineer | `tests: red <slice>` then `feat: green <slice>` | `git status` first |
Merge `--no-ff` to `main` when the ticket's acceptance tests are green.

## Blueprint for the Haiku builder (zero shared context)
Absolute paths · the types and signatures it touches · the exact table rows (name, input, expected) · allowlist and do-not-touch list · the traps below that apply · `./check.sh` · after the run `pgrep -fl <crate>` prints nothing · report format: files, gate output tail, open doubts. Run `cargo fmt` before the gate (re-wrapping tests is fine, changing a row is not). Never commit; never edit a test to make it pass. Sonnet checks the row names and count against the blueprint after green.

## Traps
Copy the relevant ones from `.claude/rules/rust.md` into every blueprint; add each new mistake there.

## Phase 2: package
README (run, test, assumptions, design in 10 lines, measured numbers, scalability, links to `DESIGN.md`, `PLAN.md`, `DECISIONS.md`, `AI_USAGE.md`) in the engineer's words; `AI_USAGE.md` with the AI's mistakes and their proof, decisions against its advice, cuts, `/usage` cost. Then: company-name and brief-sentence grep (engineer runs, empty), fresh `git clone` + `cargo build` + README commands, transcript export.
