# Plan

One branch per ticket, `t-NN-<slug>`. Commits: `tests: red <slice>` then `feat: green <slice>`, made by the engineer. Merge `--no-ff` when acceptance tests are green. Budgets are proposals, in lines (code / tests).

Before t-01: Opus attack on `DESIGN.md` and the test plan; each finding answered in `DECISIONS.md`.

| Ticket | Scope | Acceptance tests | Budget |
|---|---|---|---|
| t-01-money | `Amount` newtype, lenient parser, 4-place display, checked add and sub | string table: valid forms, `1.`, `.5`, `.`, `-0`, `+0`, `0.0000`, `1.50000`, 30 leading zeros, tab and non-breaking space inside, max value and one over; display extremes (`0.0001`, `-0.0500`, `-8.0000`, minimum and maximum `i64`); checked ops; round trip | done: 97 / 130 (budget 120 / 200) |
| t-02-parse | add `csv` (engineer approves); flexible byte records, trim all; header detection; row to event; typed parse errors naming row and field and carrying the tx id and kind for burning; zero amount rejected; any-case type | whitespace, CRLF, BOM (with headers off), short-row, no-header, empty, header-only, non-UTF-8 table; client above u16, tx above u32, non-numeric; tx id kept when another field fails | done: 321 / 376 (budget 170 / 250; header detection, burn rules and error text were bigger than estimated) |
| t-03-engine | engine, entry map, apply, lock, burn rule, rules for D4 to D9 | one table case per row of the rules table in `DESIGN.md`, each decision's example, and the multi-event cases listed in the test plan (D14 to D16) | done: 264 / 522 (budget 400 / 500, revised from 240 / 350 after t-02) |
| t-04-io-cli | sorted writer through `BufWriter` with `flush`, shared log cap of 100 with per-reason counts, exit codes 0/1/2 (exactly one path), no output on a mid-file read failure, stderr failures ignored, thin `main` | data files each shown red once (incl. D9, D15, D16, D17 and the locked-account case); `tests/cli.rs` incl. missing file, no argument, two arguments, closed stdout and closed stderr | done: 194 / 701 plus 21 data pairs (budget 200 / 300, revised from 120 / 250; the tests grew from the two-slice split, the data walker and 11 process tests) |
| t-05-edge | a fresh subagent reads only the spec and writes edge-case CSVs outside the repo; we run the binary on each and check the output by hand | every finding decided: fix (red test first), or document under `DECISIONS.md`, the README or a test | done: 3 data pairs, no engine change (tests only unless a fix is chosen) |
| t-06-gen | `examples/gen.rs`: seeded generator, same seed gives the same bytes, no new dependencies | same seed twice gives identical bytes; a different seed differs; output is valid input | 100 / 60 (revised from 80 / 40) |
| t-07-numbers-docs | 10^7-row memory and time run using the generator; confirm limits | measured numbers recorded for the README | small |

Gate for every slice: `./check.sh` re-run by the reviewer, `rust-review` on the diff, `pgrep -fl payments-engine` prints nothing.

Docs the engineer writes: README, AI_USAGE.
