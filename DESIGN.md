# Design

Reasons live in `DECISIONS.md`. Tickets live in `PLAN.md`.

## Flow
`reader (impl Read)` -> `parse: row -> Event` -> `engine.apply(event) -> Result<(), EngineError>` -> `writer (sorted, 4 places)`.
Each stage can be tested without the others. The engine never sees CSV. A row that fails to parse still hands the engine its tx id and kind (when it is a deposit or withdrawal) so the id is burned (D17). The first record is checked for a header; if it is not one it is parsed as data (D19).

## Types
| Type | Shape | Notes |
|---|---|---|
| `Amount` | newtype over `i64` ten-thousandths | validating parser; checked add and sub; Display always 4 places |
| `ClientId` / `TxId` | `u16` / `u32` | out-of-range input is a parse error |
| `Event` | enum: Deposit, Withdrawal, Dispute, Resolve, Chargeback | amount only on the first two |
| `Account` | available, held (signed), locked | total is computed, never stored |
| `Entry` | enum: Deposit{owner, amount, dispute state}, Withdrawal, Rejected | one `HashMap<u32, Entry>` for every id |
| `Dispute` | Undisputed, Open, Resolved, ChargedBack | one dispute per tx, ever |

## Rules
| Event | Effect | Ignored or rejected when |
|---|---|---|
| deposit | available += a | amount invalid or zero (parser), or not positive (engine, D28); id already used; account locked |
| withdrawal | available -= a | as deposit, or available < a |
| dispute | available -= a, held += a | unknown id; not a deposit; wrong client; already disputed once; account locked |
| resolve | held -= a, available += a; state Resolved | unknown id; wrong client; not open; locked |
| chargeback | held -= a; account locked; state ChargedBack | unknown id; wrong client; not open; locked |

Every rejected deposit or withdrawal row with a parseable id records that id as Rejected, only if the id is not already in the map (D15).
A rejected event changes nothing. A dispute computes available and held, then stores both.
A deposit is also rejected if available + held + amount would not fit in `i64` (D16). The engine refuses an amount that is not positive, burning the id and opening no account (D28). A total that does not fit makes the run fail and nothing is written (D16).
An account is opened by a deposit or withdrawal row that reaches the balance checks, even if it then fails there; a row refused as a duplicate tx id opens none; dispute, resolve and chargeback for an unknown client leave no trace (D14).
Available may go negative after a dispute. Invariants the rule tables assert: total = available + held, held >= 0.

## Output
Header `client,available,held,total,locked`, no spaces, rows sorted by client, four decimals, `true` or `false` for locked (D12, D22). Empty input gives the header line only.

## Errors
Three error enums (`AmountError`, `ParseError`, `EngineError`), each implementing `std::error::Error` and naming the row, field, client or tx. Variants are matched in tests, never messages. Engine refusals are logged with their row number. Overflow rejects the row and the run continues. Row errors are logged up to a shared cap of 100 lines, then counted per reason: bad, failed, ignored, needs review (D20). Exit codes: 0 done, 1 I/O failure (open, read or write; a mid-file read failure writes no accounts), 2 unless exactly one input path (D21). Failures writing to stderr are ignored. Only an I/O error ends the run; short rows and non-UTF-8 fields are row errors (D18).

## Assumptions (all to appear in the README in the engineer's words)
- Dispute, resolve and chargeback rows from a client who does not own the tx are treated like an unknown tx.
- Unknown type: skipped and logged, no id burned.
- A locked account needs a human; the program never unlocks it.
- Quotes in the input are ordinary characters (D18); a comma inside quotes is not supported. Empty fields after the fourth are ignored (D24).
- UNVERIFIED: how an automatic scorer treats the exit code.

## Test plan
1. Unit tables per module: amount strings, field parsing, engine rules (one named case per row of the rules table above, plus each decision's example). The engine tables also hold multi-event cases: double resolve, chargeback after resolve, two disputes then one chargeback, a duplicate deposit followed by a dispute, a wrong-client dispute followed by the owner's dispute, and an overflowing dispute (D15, D16).
2. `tests/data/<behaviour>.in.csv` + `.out.csv`, hand computed, each shown failing once by changing one character.
3. `tests/cli.rs` runs the real binary: success, bad rows, missing file, no argument, two arguments, a directory as input, the log cap, a BOM, a missing header, closed stdout and closed stderr.
4. Edge-case pass (t-05-edge): a fresh subagent reads only the spec and writes edge-case CSVs outside the repo. We check each one and decide: fix the code, or document under decisions, the README or tests.
5. Measured run on 10^7 rows for memory and time (recorded in D10), using the seeded generator `examples/gen.rs`, plus an opt-in 10^7-row stream test.
6. Opus attack on this file and this plan before t-01: which bug still passes these tests?
