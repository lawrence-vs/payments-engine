# Plan

One branch per ticket, `t-NN-<slug>`. Commits: `tests: red <slice>` then `feat: green <slice>`, made by the engineer. Merge `--no-ff` when acceptance tests are green. Budgets are proposals, in lines (code / tests).

Before t-01: Opus attack on `DESIGN.md` and the test plan; each finding answered in `DECISIONS.md`.

| Ticket | Scope | Acceptance tests | Budget |
|---|---|---|---|
| t-01-money | `Amount` newtype, lenient parser, 4-place display, checked add and sub | string table: valid forms, `1.`, `.5`, rejects, max value and one over; display extremes (`0.0001`, `-0.0500`, `-8.0000`) | 120 / 150 |
| t-02-parse | add `csv` (engineer approves); row to event; typed parse errors naming row and field; trim, CRLF, BOM, short rows, any-case type | whitespace, CRLF, BOM, short-row table; client above u16, tx above u32, non-numeric | 150 / 200 |
| t-03-engine | engine, entry map, apply, lock, burn rule, rules for D4 to D9 | one case per rules-table row; invariant test over seeded random events | 220 / 350 |
| t-04-io-cli | sorted writer through `BufWriter` with `flush`, bad-row cap and summary, exit codes, thin `main` | data files each shown red once; `tests/cli.rs` incl. missing file and closed stdout | 100 / 200 |
| t-05-numbers-docs | 10^7-row memory and time run; confirm limits | measured numbers recorded for the README | small |

Gate for every slice: `./check.sh` re-run by the reviewer, `rust-review` on the diff, `pgrep -fl payments-engine` prints nothing.

Docs the engineer writes: README, AI_USAGE.
