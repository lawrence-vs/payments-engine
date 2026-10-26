# payments-engine

Rules and decisions: `DESIGN.md`, `DECISIONS.md`, `PLAN.md` (read before changing behaviour).

## Commands
- Gate, before every commit: `./check.sh` (fmt, clippy -D warnings, test, doc, audit). CI runs the same file.
- One test: `cargo test <name>`.

## Contract
- stdout: accounts CSV only, 4 decimals
- stderr: logs
- exit 0 done, 1 I/O failure, 2 no input path
- input: type, client (u16), tx (u32), amount (≤ 4 places), spaces allowed

## Code rules
In `.claude/rules/rust.md` (loads with any `.rs` file).
- Comments: none by default; one-line `///` only on a `pub` item whose name does not say what it does; one-line `//` only for a non-obvious why; none on trait impls or simple getters; no multi-line blocks. Copy this into every blueprint.

## Tests
- Table-driven: named cases, inputs, expected output. Match errors by variant, never by message. A loop collects every failure before one assert.
- Unit tables in `src/`; `tests/data/<behaviour>.in.csv` + `.out.csv` with hand-computed output, walked by `tests/data_files.rs`; `tests/cli.rs` runs the real binary.
- Table first, red for the right reason (wrong value, not a missing symbol), then code. A data file is shown able to fail once.
- Never fix a failing test by editing the test. `cargo fmt` re-wrapping a test is fine.

## Docs
- README short: run, test, design in 10 lines, numbers; link `DESIGN.md`, `DECISIONS.md`, `PLAN.md`, `AI_USAGE.md` for the rest.
- README and AI_USAGE are written by the engineer. Claude may suggest structure and list unbacked claims, never write the prose.

## Workflow
- `start-workflow` skill: Plan -> Red -> Act -> Verify per slice; `test-first-loop` memo before code; `rust-review` on every diff.
- Sonnet main (plans, reviews, re-runs the gate itself), Haiku subagent builds from a blueprint, Opus for one design attack and the final review.
- Facts are run before trusted; unchecked claims are UNVERIFIED. Policies are the engineer's, in `DECISIONS.md` with rejected options.
- No MCP servers or connectors: `/mcp` must list nothing before the first prompt.
- Branch per ticket `t-NN-<slug>`; `tests: red <slice>` then `feat: green <slice>`. Claude can do the git steps after the engineer says commit; the engineer reads every diff before a merge. `git status` before every commit.

## Do not
- Put the task brief, its text, or any company name in this repo.
- Let an IDE agent write in the repo.
