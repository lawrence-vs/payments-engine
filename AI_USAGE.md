# AI usage

I used Claude Code for the build. Full transcripts are attached to the submission email.

## Models and roles

| Model                                    | Role                                                                                                                             |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Sonnet 5.5 (main session, medium effort) | planning, blueprints for each ticket, re-running `./check.sh` itself, `rust-review` on every diff, git steps after I said commit |
| Haiku 5.5 (7 subagent runs)              | built each ticket from a written blueprint: failing tests first, then the code                                                   |
| Opus 5.5 (3 subagent runs)               | an attack on the design and test plan before any code, an edge-case check from the spec alone, and a final review                |

I made every policy decision, one at a time, with the options and a test run in front of me. Each is in `DECISIONS.md` with my reason and the options I rejected. I read each ticket's diff before it was merged.

## Mistakes the AI made, and how they were caught

| Mistake                                                                                 | Caught by                                                                        | Fix                                                                                      |
| --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Two parser test rows expected 1 unit for the amount `1` (it is 10000)                   | the code was right and the rows failed; checked against the earlier amount table | corrected in its own commit                                                              |
| No test pinned the burned id of a withdrawal on a locked account                        | the final review                                                                 | added the test row                                                                       |
| A doc comment on every item, most repeating the item's name                             | my reading of the code                                                           | a one-line, only-when-needed comment rule in `.claude/rules/rust.md` and every blueprint |
| A stray `"` made the csv reader swallow every later row, exit 0                         | the final review                                                                 | quoting turned off; only that row is bad                                                 |
| The memory estimate was too low (0.4 GiB predicted at 10^7 rows, about 0.8 GB measured) | the measured run                                                                 | README uses the measured numbers                                                         |
| Sonnet kept labelling options "Recommended" and saying which it would pick              | me                                                                               | told it to give evidence only and let me decide                                          |

## Decisions I made against the AI's suggestion

- A locked account refuses every later row, not only deposits and withdrawals (D4).
- A resolved dispute is final; a second dispute on the same tx is refused (D6).
- Every refused deposit or withdrawal uses up its tx id, not only malformed ones (D2c).
- A duplicate tx id opens no account (D14), reversing how the first data files were written.
- A stray quote spoils one row instead of ending the run; an empty trailing field is tolerated (F1, F14).

## How I verified

`./check.sh` (fmt, clippy `-D warnings`, tests, docs, `cargo audit`) on every passing commit and before every merge, re-run by the main session rather than taken from the builder's report. Failing tests were committed before each fix. After each batch of data files, one changed digit showed the data-file test can fail. An independent Opus agent wrote 52 edge cases from the spec alone: all 33 that the spec decides match, with no crashes. Time and memory measured on 10^7 rows.

## Notes

- Auto mode handled permission prompts for part of the build; I reviewed each ticket's diff before merging.
- Time: Planning took longer than expected because each decision was taken one at a time with a test run.
