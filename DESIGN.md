# Design

Reasons live in `DECISIONS.md`. Tickets live in `PLAN.md`.

## Flow
`reader (impl Read)` -> `parse: row -> Event` -> `engine.apply(event) -> Result<Outcome, EngineError>` -> `writer (sorted, 4 places)`.
Each stage can be tested without the others. The engine never sees CSV.

## Types
| Type | Shape | Notes |
|---|---|---|
| `Amount` | newtype over `i64` ten-thousandths | validating parser; checked add and sub; Display always 4 places |
| `ClientId` / `TxId` | `u16` / `u32` | out-of-range input is a parse error |
| `Event` | enum: Deposit, Withdrawal, Dispute, Resolve, Chargeback | amount only on the first two |
| `Account` | available, held (signed), locked | total is computed, never stored |
| `Entry` | enum: Deposit{owner, amount, dispute state}, Withdrawal, Rejected | one `HashMap<u32, Entry>` for every id |
| `DisputeState` | None, Open, Resolved, ChargedBack | one dispute per tx, ever |

## Rules
| Event | Effect | Ignored or rejected when |
|---|---|---|
| deposit | available += a | amount invalid or zero; id already used; account locked |
| withdrawal | available -= a | as deposit, or available < a |
| dispute | available -= a, held += a | unknown id; not a deposit; wrong client; already disputed once; account locked |
| resolve | held -= a, available += a; state Resolved | unknown id; wrong client; not open; locked |
| chargeback | held -= a; account locked; state ChargedBack | unknown id; wrong client; not open; locked |

Every rejected deposit or withdrawal row with a parseable id records that id as Rejected.
Available may go negative after a dispute. Invariants checked in tests: total = available + held, held >= 0.

## Errors
One error enum implementing `std::error::Error`, with a category and the row and field named. Variants are matched in tests, never messages. Overflow rejects the row and the run continues. Row errors are logged (first 100, then counted); open, read and write failures end the run with exit 1.

## Assumptions (all to appear in the README in the engineer's words)
- Dispute, resolve and chargeback rows from a client who does not own the tx are treated like an unknown tx.
- Unknown type: skipped and logged, no id burned.
- A locked account needs a human; the program never unlocks it.
- UNVERIFIED: BOM handling in `csv`; scorer exit-code handling; entry size and memory figures.

## Test plan
1. Unit tables per module: amount strings, field parsing, engine rules (one named case per row of the rules table above, plus each decision's example).
2. A seeded random-operation test checking the two invariants after every event, with no new dependency.
3. `tests/data/<behaviour>.in.csv` + `.out.csv`, hand computed, each shown failing once by changing one character.
4. `tests/cli.rs` runs the real binary: success, missing file, no argument, closed stdout.
5. Measured run on 10^7 rows for memory and time, recorded in the README.
6. Opus attack on this file and this plan before t-01: which bug still passes these tests?
