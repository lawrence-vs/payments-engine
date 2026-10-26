use super::*;
use crate::{Amount, ClientId, Event, TxId};

const M: i64 = i64::MAX;

enum Step {
    Apply(Event),
    Burn(u32),
}

fn dep(client: u16, tx: u32, units: i64) -> Event {
    Event::Deposit {
        client: ClientId::new(client),
        tx: TxId::new(tx),
        amount: Amount::from_units(units),
    }
}

fn wd(client: u16, tx: u32, units: i64) -> Event {
    Event::Withdrawal {
        client: ClientId::new(client),
        tx: TxId::new(tx),
        amount: Amount::from_units(units),
    }
}

fn disp(client: u16, tx: u32) -> Event {
    Event::Dispute { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn res(client: u16, tx: u32) -> Event {
    Event::Resolve { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn cb(client: u16, tx: u32) -> Event {
    Event::Chargeback { client: ClientId::new(client), tx: TxId::new(tx) }
}

fn sum_units(available: Amount, held: Amount) -> Option<i64> {
    available.checked_add(held).map(Amount::units)
}

type Case<'a> = (&'a str, &'a [Step], &'a [Result<(), EngineError>], &'a [(u16, i64, i64, bool)]);

#[test]
fn engine_table() {
    let cases: &[Case<'_>] = &[
        ("deposit_credits", &[Step::Apply(dep(1, 1, 100))], &[Ok(())], &[(1, 100, 0, false)]),
        (
            "withdraw_exact",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(wd(1, 2, 100))],
            &[Ok(()), Ok(())],
            &[(1, 0, 0, false)],
        ),
        (
            "withdraw_one_over",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(wd(1, 2, 101))],
            &[
                Ok(()),
                Err(EngineError::InsufficientFunds { client: ClientId::new(1), tx: TxId::new(2) }),
            ],
            &[(1, 100, 0, false)],
        ),
        (
            "failed_withdrawal_burns_id",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(wd(1, 2, 101)), Step::Apply(dep(1, 2, 5))],
            &[
                Ok(()),
                Err(EngineError::InsufficientFunds { client: ClientId::new(1), tx: TxId::new(2) }),
                Err(EngineError::DuplicateTx { tx: TxId::new(2) }),
            ],
            &[(1, 100, 0, false)],
        ),
        (
            "withdraw_on_empty_opens_account",
            &[Step::Apply(wd(7, 1, 5))],
            &[Err(EngineError::InsufficientFunds { client: ClientId::new(7), tx: TxId::new(1) })],
            &[(7, 0, 0, false)],
        ),
        (
            "clients_interleaved_sorted",
            &[Step::Apply(dep(2, 1, 50)), Step::Apply(dep(1, 2, 70)), Step::Apply(wd(2, 3, 20))],
            &[Ok(()), Ok(()), Ok(())],
            &[(1, 70, 0, false), (2, 30, 0, false)],
        ),
        (
            "duplicate_deposit_ignored",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(dep(1, 1, 5))],
            &[Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(1) })],
            &[(1, 10, 0, false)],
        ),
        (
            "duplicate_then_dispute_keeps_original",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(dep(1, 1, 5)), Step::Apply(disp(1, 1))],
            &[Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(1) }), Ok(())],
            &[(1, 0, 10, false)],
        ),
        (
            "duplicate_after_dispute_then_chargeback_locks",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(dep(1, 1, 5)),
                Step::Apply(cb(1, 1)),
            ],
            &[Ok(()), Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(1) }), Ok(())],
            &[(1, 0, 0, true)],
        ),
        (
            "deposit_reusing_withdrawal_id",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(wd(1, 5, 4)), Step::Apply(dep(1, 5, 7))],
            &[Ok(()), Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(5) })],
            &[(1, 6, 0, false)],
        ),
        (
            "withdrawal_reusing_deposit_id_keeps_the_deposit",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(wd(1, 1, 40)), Step::Apply(disp(1, 1))],
            &[Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(1) }), Ok(())],
            &[(1, 0, 100, false)],
        ),
        (
            "burned_id_blocks_deposit_and_opens_no_account",
            &[Step::Burn(5), Step::Apply(dep(1, 5, 10))],
            &[Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(5) })],
            &[],
        ),
        (
            "duplicate_deposit_from_new_client_opens_no_account",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(dep(2, 1, 5))],
            &[Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(1) })],
            &[(1, 10, 0, false)],
        ),
        (
            "duplicate_withdrawal_from_new_client_opens_no_account",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(wd(2, 1, 5))],
            &[Ok(()), Err(EngineError::DuplicateTx { tx: TxId::new(1) })],
            &[(1, 10, 0, false)],
        ),
        (
            "burn_keeps_existing_entry",
            &[Step::Apply(dep(1, 1, 10)), Step::Burn(1), Step::Apply(disp(1, 1))],
            &[Ok(()), Ok(()), Ok(())],
            &[(1, 0, 10, false)],
        ),
        (
            "dispute_holds",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(disp(1, 1))],
            &[Ok(()), Ok(())],
            &[(1, 0, 100, false)],
        ),
        (
            "resolve_releases",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(disp(1, 1)), Step::Apply(res(1, 1))],
            &[Ok(()), Ok(()), Ok(())],
            &[(1, 100, 0, false)],
        ),
        (
            "chargeback_removes_and_locks",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(disp(1, 1)), Step::Apply(cb(1, 1))],
            &[Ok(()), Ok(()), Ok(())],
            &[(1, 0, 0, true)],
        ),
        (
            "dispute_unknown_tx_leaves_no_trace",
            &[Step::Apply(disp(9, 99))],
            &[Err(EngineError::UnknownTx { tx: TxId::new(99) })],
            &[],
        ),
        (
            "resolve_unknown_tx_leaves_no_trace",
            &[Step::Apply(res(1, 5))],
            &[Err(EngineError::UnknownTx { tx: TxId::new(5) })],
            &[],
        ),
        (
            "chargeback_unknown_tx_leaves_no_trace",
            &[Step::Apply(cb(1, 5))],
            &[Err(EngineError::UnknownTx { tx: TxId::new(5) })],
            &[],
        ),
        (
            "resolve_not_disputed",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(res(1, 1))],
            &[Ok(()), Err(EngineError::NotUnderDispute { tx: TxId::new(1) })],
            &[(1, 100, 0, false)],
        ),
        (
            "chargeback_not_disputed",
            &[Step::Apply(dep(1, 1, 100)), Step::Apply(cb(1, 1))],
            &[Ok(()), Err(EngineError::NotUnderDispute { tx: TxId::new(1) })],
            &[(1, 100, 0, false)],
        ),
        (
            "dispute_of_withdrawal_ignored",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(wd(1, 2, 4)), Step::Apply(disp(1, 2))],
            &[Ok(()), Ok(()), Err(EngineError::NotDisputable { tx: TxId::new(2) })],
            &[(1, 6, 0, false)],
        ),
        (
            "dispute_of_rejected_id_ignored",
            &[Step::Apply(wd(1, 1, 5)), Step::Apply(disp(1, 1))],
            &[
                Err(EngineError::InsufficientFunds { client: ClientId::new(1), tx: TxId::new(1) }),
                Err(EngineError::NotDisputable { tx: TxId::new(1) }),
            ],
            &[(1, 0, 0, false)],
        ),
        (
            "burned_id_not_disputable_no_account",
            &[Step::Burn(5), Step::Apply(disp(1, 5))],
            &[Ok(()), Err(EngineError::NotDisputable { tx: TxId::new(5) })],
            &[],
        ),
        (
            "dispute_wrong_client",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(disp(2, 1))],
            &[
                Ok(()),
                Err(EngineError::ClientMismatch { client: ClientId::new(2), tx: TxId::new(1) }),
            ],
            &[(1, 10, 0, false)],
        ),
        (
            "resolve_wrong_client",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(disp(1, 1)), Step::Apply(res(2, 1))],
            &[
                Ok(()),
                Ok(()),
                Err(EngineError::ClientMismatch { client: ClientId::new(2), tx: TxId::new(1) }),
            ],
            &[(1, 0, 10, false)],
        ),
        (
            "chargeback_wrong_client",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(disp(1, 1)), Step::Apply(cb(2, 1))],
            &[
                Ok(()),
                Ok(()),
                Err(EngineError::ClientMismatch { client: ClientId::new(2), tx: TxId::new(1) }),
            ],
            &[(1, 0, 10, false)],
        ),
        (
            "wrong_client_dispute_does_not_use_up_the_rule",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(disp(2, 1)), Step::Apply(disp(1, 1))],
            &[
                Ok(()),
                Err(EngineError::ClientMismatch { client: ClientId::new(2), tx: TxId::new(1) }),
                Ok(()),
            ],
            &[(1, 0, 10, false)],
        ),
        (
            "second_dispute_while_open",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(disp(1, 1)), Step::Apply(disp(1, 1))],
            &[Ok(()), Ok(()), Err(EngineError::AlreadyDisputed { tx: TxId::new(1) })],
            &[(1, 0, 10, false)],
        ),
        (
            "dispute_after_resolve_refused",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(res(1, 1)),
                Step::Apply(disp(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Err(EngineError::AlreadyDisputed { tx: TxId::new(1) })],
            &[(1, 10, 0, false)],
        ),
        (
            "resolve_twice",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(res(1, 1)),
                Step::Apply(res(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Err(EngineError::NotUnderDispute { tx: TxId::new(1) })],
            &[(1, 10, 0, false)],
        ),
        (
            "chargeback_after_resolve",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(res(1, 1)),
                Step::Apply(cb(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Err(EngineError::NotUnderDispute { tx: TxId::new(1) })],
            &[(1, 10, 0, false)],
        ),
        (
            "two_disputes_one_chargeback",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(dep(1, 2, 5)),
                Step::Apply(disp(1, 1)),
                Step::Apply(disp(1, 2)),
                Step::Apply(cb(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Ok(()), Ok(())],
            &[(1, 0, 5, true)],
        ),
        (
            "resolve_after_chargeback_refused",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
                Step::Apply(res(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Err(EngineError::Locked { client: ClientId::new(1) })],
            &[(1, 0, 0, true)],
        ),
        (
            "chargeback_twice_refused",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
                Step::Apply(cb(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Err(EngineError::Locked { client: ClientId::new(1) })],
            &[(1, 0, 0, true)],
        ),
        (
            "dispute_after_chargeback_refused",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
                Step::Apply(disp(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Err(EngineError::Locked { client: ClientId::new(1) })],
            &[(1, 0, 0, true)],
        ),
        (
            "deposit_on_locked_refused_and_burned",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
                Step::Apply(dep(1, 2, 5)),
                Step::Apply(dep(1, 2, 5)),
            ],
            &[
                Ok(()),
                Ok(()),
                Ok(()),
                Err(EngineError::Locked { client: ClientId::new(1) }),
                Err(EngineError::DuplicateTx { tx: TxId::new(2) }),
            ],
            &[(1, 0, 0, true)],
        ),
        (
            "withdrawal_on_locked_refused",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(dep(1, 2, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
                Step::Apply(wd(1, 3, 5)),
            ],
            &[
                Ok(()),
                Ok(()),
                Ok(()),
                Ok(()),
                Err(EngineError::Locked { client: ClientId::new(1) }),
            ],
            &[(1, 10, 0, true)],
        ),
        (
            "open_dispute_stays_open_on_locked",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(dep(1, 2, 5)),
                Step::Apply(disp(1, 1)),
                Step::Apply(disp(1, 2)),
                Step::Apply(cb(1, 1)),
                Step::Apply(res(1, 2)),
                Step::Apply(cb(1, 2)),
                Step::Apply(dep(1, 3, 1)),
            ],
            &[
                Ok(()),
                Ok(()),
                Ok(()),
                Ok(()),
                Ok(()),
                Err(EngineError::Locked { client: ClientId::new(1) }),
                Err(EngineError::Locked { client: ClientId::new(1) }),
                Err(EngineError::Locked { client: ClientId::new(1) }),
            ],
            &[(1, 0, 5, true)],
        ),
        (
            "lock_is_per_client",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
                Step::Apply(dep(2, 2, 7)),
            ],
            &[Ok(()), Ok(()), Ok(()), Ok(())],
            &[(1, 0, 0, true), (2, 7, 0, false)],
        ),
        (
            "dispute_after_spend_goes_negative",
            &[Step::Apply(dep(1, 1, 10)), Step::Apply(wd(1, 2, 8)), Step::Apply(disp(1, 1))],
            &[Ok(()), Ok(()), Ok(())],
            &[(1, -8, 10, false)],
        ),
        (
            "chargeback_after_spend_total_negative",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(wd(1, 2, 8)),
                Step::Apply(disp(1, 1)),
                Step::Apply(cb(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Ok(())],
            &[(1, -8, 0, true)],
        ),
        (
            "resolve_after_negative_dispute",
            &[
                Step::Apply(dep(1, 1, 10)),
                Step::Apply(wd(1, 2, 8)),
                Step::Apply(disp(1, 1)),
                Step::Apply(res(1, 1)),
            ],
            &[Ok(()), Ok(()), Ok(()), Ok(())],
            &[(1, 2, 0, false)],
        ),
        (
            "deposit_overflow_burns_id",
            &[Step::Apply(dep(1, 1, M)), Step::Apply(dep(1, 2, 1)), Step::Apply(dep(1, 2, 1))],
            &[
                Ok(()),
                Err(EngineError::Overflow { client: ClientId::new(1), tx: TxId::new(2) }),
                Err(EngineError::DuplicateTx { tx: TxId::new(2) }),
            ],
            &[(1, M, 0, false)],
        ),
        (
            "deposit_total_overflow_via_held",
            &[Step::Apply(dep(1, 1, M)), Step::Apply(disp(1, 1)), Step::Apply(dep(1, 2, 1))],
            &[
                Ok(()),
                Ok(()),
                Err(EngineError::Overflow { client: ClientId::new(1), tx: TxId::new(2) }),
            ],
            &[(1, 0, M, false)],
        ),
        (
            "dispute_is_atomic_on_overflow",
            &[
                Step::Apply(dep(1, 1, M)),
                Step::Apply(wd(1, 2, M)),
                Step::Apply(dep(1, 3, 1)),
                Step::Apply(wd(1, 4, 1)),
                Step::Apply(disp(1, 1)),
                Step::Apply(disp(1, 3)),
                Step::Apply(res(1, 3)),
            ],
            &[
                Ok(()),
                Ok(()),
                Ok(()),
                Ok(()),
                Ok(()),
                Err(EngineError::Overflow { client: ClientId::new(1), tx: TxId::new(3) }),
                Err(EngineError::NotUnderDispute { tx: TxId::new(3) }),
            ],
            &[(1, -M, M, false)],
        ),
    ];

    let mut failures = Vec::new();
    for (name, steps, expected_results, expected_accounts) in cases {
        let mut engine = Engine::new();
        let results: Vec<Result<(), EngineError>> = steps
            .iter()
            .map(|step| match step {
                Step::Apply(event) => engine.apply(*event),
                Step::Burn(tx) => {
                    engine.burn(TxId::new(*tx));
                    Ok(())
                }
            })
            .collect();
        let accounts: Vec<(u16, i64, i64, bool)> = engine
            .accounts()
            .map(|(client, account)| {
                (
                    client.get(),
                    account.available().units(),
                    account.held().units(),
                    account.locked(),
                )
            })
            .collect();
        if results.as_slice() != *expected_results || accounts.as_slice() != *expected_accounts {
            failures.push(format!(
                "{name}: results {results:?}, want {expected_results:?}; accounts {accounts:?}, want {expected_accounts:?}"
            ));
        }
        for (client, account) in engine.accounts() {
            if sum_units(account.available(), account.held()) != Some(account.total().units())
                || account.held().units() < 0
            {
                failures.push(format!(
                    "{name}: client {}: total {:?} is not available plus held, or held is negative",
                    client.get(),
                    account.total()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
