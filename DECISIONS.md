# Decisions

Policies are the engineer's. Each row: what we do, why (engineer's words), rejected options. Status UNVERIFIED means a fact or assumption not yet run.

## D1 CSV layer
- **Do:** `csv` crate only, no serde. Rows become typed values by our own field parsing.
- **Why:** I can rely on typed rows for error matching and type safety enforced, instead of parsing strings.
- **Rejected:** `csv` + serde derive (field errors arrive as strings); no crates (we would own quoting, CRLF, BOM).
- **UNVERIFIED:** whether `csv` strips a BOM; transitive dependency count. t-02 tests the BOM.

## D2 Amount format
- **Do:** lenient parse into whole ten-thousandths. Accept `1.` and `.5`. No sign accepted. More than 4 places, `1e3`, `1,5`, NaN, empty and overflow are rejected.
- **Why:** We can be lenient, a sign is not needed as the action is inferred from the deposit / withdrawal type, so we accept different formats to be a bit more robust. We state this in the design and in the guidelines for the CSV upload format.
- **Rejected:** strict (only digits and one optional fraction); a decimal crate.
- **UNVERIFIED:** my reading that "no sign needed" means `+1` stays rejected; whether the scorer ever sends `.5` or `1.`.

## D2b Zero amount
- **Do:** a deposit or withdrawal of 0 is rejected and logged.
- **Why:** It does not move money and can be used to lock an account unnecessarily. We can consider it an error because it does not make logical sense.
- **Rejected:** accept zero as a no-op; accept zero but make it not disputable.
- **Evidence:** a zero deposit, disputed then charged back, locked the account and blocked a later real deposit of 10.

## D2c Rejected rows burn their tx id
- **Do:** every rejected deposit or withdrawal row whose tx id parsed records that id as used. A later row with the same id is refused.
- **Why:** Every rejected row is added to the transactions already processed and cannot be replayed. The partner should create a new tx, so our system stays idempotent and we cannot reapply a withdrawal twice with different outcomes. A row cannot be edited and still accepted because it had different properties.
- **Rejected:** burn only invalid-content rows (a failed withdrawal could be retried); burn nothing.
- **UNVERIFIED:** scope is deposit and withdrawal rows only; dispute, resolve and chargeback rows reference an existing id and burn nothing.

## D3 Bad rows and exit code
- **Do:** log the first 100 skipped rows (row number, field, category; never the raw text), then count the rest and print one summary line to stderr. Exit 0 if the file was read; exit 1 only for open, read or write failure.
- **Why:** 100 rows is enough to diagnose the issue causing skipped rows and see the pattern. If the system scales it is better to raise it; for this design constraint 100 is fine.
- **Rejected:** log 1000; exit non-zero when any row was skipped.
- **Evidence:** about 52 MB of stderr per million bad rows if all logged, about 5 KB at cap 100.
- **UNVERIFIED:** how the automatic scorer treats a non-zero exit.

## D4 Locked account
- **Do:** after a chargeback, the account accepts no automatic events: no deposit, withdrawal, dispute, resolve or chargeback.
- **Why:** On a frozen account we block so that a human can review and manually resolve. We do not allow any more automatic events on the account. It is safer for us and avoids fraudulent transactions or illicit behaviour.
- **Rejected:** block deposits and withdrawals only; set the flag and process everything.
- **Consequence:** other open disputes on a locked account stay open until a human acts.

## D5 Which transactions can be disputed
- **Do:** deposits only. A dispute that names a withdrawal id is ignored and logged.
- **Why:** (engineer to fill in)
- **Rejected:** deposits and withdrawals; withdrawals only.
- **Evidence:** deposit 10, withdraw 4, dispute the withdrawal, chargeback: total ends 2 instead of 6 (the client is debited twice).

## D6 One dispute per transaction
- **Do:** a transaction can be disputed once, ever. A second dispute (while open, or after a resolve) changes no balance and is reported as an error for human review. Handling is idempotent. Charged back and resolved are final.
- **Why:** One dispute per transaction avoids duplicate replays of disputes. The second is an error action and needs human review. One claim is one payment back for it in our logic.
- **Rejected:** allow a re-dispute after a resolve.
- **UNVERIFIED:** "human review" means a distinct error on stderr, not an account lock.

## D7 Negative available balance
- **Do:** a dispute may take available below zero (signed balances). The chargeback still applies and locks the account. No extra lock on a negative dispute.
- **Why:** Available decreases by the disputed amount, so a negative balance is possible and should show once disputed, so the human reviewing the chargeback claim sees that the outcome is a negative balance. The negative total tells the reviewer we will make a loss on that account and that it could be a deposit, use, dispute and withdrawal fraud. Only a chargeback locks.
- **Rejected:** ignore a dispute that would go negative; hold only the available part.
- **Evidence:** deposit 10, withdraw 8, dispute the deposit: available -8, held 10; after chargeback total -8.

## D8 Client mismatch
- **Do:** a dispute, resolve or chargeback from a client that does not own the transaction is ignored and logged, the same as an unknown tx.
- **Why:** A client can only dispute their own transaction. Otherwise it would be chaotic if client B could freeze client A's funds with A's tx id. Even if an id leaks, clients must not be able to act on other accounts. We treat it as a mismatch or partner error, the same for resolves and chargebacks.
- **Rejected:** apply to the owner; apply to the sender.

## D9 Duplicate or reused tx id
- **Do:** a duplicate id, including a deposit that reuses a withdrawal's id, is ignored and logged. The first row wins.
- **Why:** Tx ids are globally unique, so a reuse or duplicate id is a partner error and can be logged.
- **Rejected:** trust uniqueness and check deposits only; a separate set for withdrawal ids.
- **Evidence:** with deposits checked only, a reused id was credited (available 13 instead of 6).

## D10 Memory for tracked ids
- **Do:** one `HashMap<u32, Entry>` holding every id used and what it was used for: a deposit (owner, amount, dispute state), a withdrawal, or a rejected row. The README states the maximum number of transactions this design can hold.
- **Why:** Only one map is needed to keep track of ids used and what they were used for. It is easiest to review and it covers disputes. We can use it to state the limitation of this design. A section on future options can argue for a bitset and a dispute window for larger systems, depending on how users use or abuse the system. We do not want to over-design if it is not necessary.
- **Rejected:** bitset of ids plus a deposit map; a hard entry limit that stops the run.
- **UNVERIFIED:** entry size (assumed 24 bytes) and memory at 10^7 rows; estimates 0.4 GiB at 10^7, 3 GiB at 10^8, about 200 GiB if all u32 ids are used.

## D11 Shape
- **Do:** synchronous. Parse a row into an event, `apply(engine, event)` returns a result, then write. The reader is generic over `Read`. Async and sharding are described only in the README.
- **Why:** (engineer to fill in)
- **Rejected:** one loop doing everything; trait-based processors.

## D12 Output
- **Do:** rows sorted by client id; every amount printed with exactly four decimals (`0.0001`, `-0.0500`, `-8.0000`).
- **Why:** Four places is the same precision as the input, and sorting makes every run identical and easy to compare byte for byte, and to test the padding edge cases `0.0001` and `-0.0500`.
- **Rejected:** trimmed zeros; hash order.

## D13 Type column case
- **Do:** accept any case after trimming, matched with `eq_ignore_ascii_case` against each name. No lowercase copy is allocated.
- **Why:** Be lenient: if the text matches once lowercased, do not risk losing deposits that should be allocated, and lose money.
- **Rejected:** lowercase only.

## Open for the Opus attack
Questions to answer here once raised: locked account with open disputes; burned ids and memory growth under a flood of bad rows; one-dispute rule versus a resolve that arrives after a chargeback.
