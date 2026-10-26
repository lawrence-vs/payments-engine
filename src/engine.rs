use crate::{Amount, ClientId, Event, TxId};
use std::collections::{BTreeMap, HashMap};
use std::fmt;

pub struct Engine {
    accounts: BTreeMap<ClientId, Account>,
    entries: HashMap<TxId, Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    available: Amount,
    held: Amount,
    locked: bool,
}

enum Entry {
    Deposit { client: ClientId, amount: Amount, dispute: Dispute },
    Withdrawal,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dispute {
    Undisputed,
    Open,
    Resolved,
    ChargedBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineError {
    Locked { client: ClientId },
    DuplicateTx { tx: TxId },
    InsufficientFunds { client: ClientId, tx: TxId },
    Overflow { client: ClientId, tx: TxId },
    UnknownTx { tx: TxId },
    NotDisputable { tx: TxId },
    NotUnderDispute { tx: TxId },
    ClientMismatch { client: ClientId, tx: TxId },
    AlreadyDisputed { tx: TxId },
}

const _: () = assert!(std::mem::size_of::<Entry>() == 16);

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self { accounts: BTreeMap::new(), entries: HashMap::new() }
    }

    pub fn apply(&mut self, event: Event) -> Result<(), EngineError> {
        match event {
            Event::Deposit { client, tx, amount } => self.deposit(client, tx, amount),
            Event::Withdrawal { client, tx, amount } => self.withdraw(client, tx, amount),
            Event::Dispute { client, tx } => self.dispute(client, tx),
            Event::Resolve { client, tx } => self.resolve(client, tx),
            Event::Chargeback { client, tx } => self.chargeback(client, tx),
        }
    }

    pub fn burn(&mut self, tx: TxId) {
        burn_id(&mut self.entries, tx);
    }

    pub fn accounts(&self) -> impl Iterator<Item = (ClientId, &Account)> {
        self.accounts.iter().map(|(client, account)| (*client, account))
    }

    fn deposit(&mut self, client: ClientId, tx: TxId, amount: Amount) -> Result<(), EngineError> {
        let account = self.accounts.entry(client).or_insert_with(Account::new);
        if self.entries.contains_key(&tx) {
            return Err(EngineError::DuplicateTx { tx });
        }
        if account.locked {
            burn_id(&mut self.entries, tx);
            return Err(EngineError::Locked { client });
        }
        let Some(available) = account
            .available
            .checked_add(amount)
            .filter(|available| available.checked_add(account.held).is_some())
        else {
            burn_id(&mut self.entries, tx);
            return Err(EngineError::Overflow { client, tx });
        };
        account.available = available;
        self.entries.insert(tx, Entry::Deposit { client, amount, dispute: Dispute::Undisputed });
        Ok(())
    }

    fn withdraw(&mut self, client: ClientId, tx: TxId, amount: Amount) -> Result<(), EngineError> {
        let account = self.accounts.entry(client).or_insert_with(Account::new);
        if self.entries.contains_key(&tx) {
            return Err(EngineError::DuplicateTx { tx });
        }
        if account.locked {
            burn_id(&mut self.entries, tx);
            return Err(EngineError::Locked { client });
        }
        if account.available < amount {
            burn_id(&mut self.entries, tx);
            return Err(EngineError::InsufficientFunds { client, tx });
        }
        let Some(available) = account.available.checked_sub(amount) else {
            burn_id(&mut self.entries, tx);
            return Err(EngineError::Overflow { client, tx });
        };
        account.available = available;
        self.entries.insert(tx, Entry::Withdrawal);
        Ok(())
    }

    fn dispute(&mut self, client: ClientId, tx: TxId) -> Result<(), EngineError> {
        let (owner, amount, state) = match self.entries.get(&tx) {
            None => return Err(EngineError::UnknownTx { tx }),
            Some(Entry::Deposit { client, amount, dispute }) => (*client, *amount, *dispute),
            Some(_) => return Err(EngineError::NotDisputable { tx }),
        };
        if owner != client {
            return Err(EngineError::ClientMismatch { client, tx });
        }
        // A deposit always opened its account, so this entry never creates one.
        let account = self.accounts.entry(owner).or_insert_with(Account::new);
        if account.locked {
            return Err(EngineError::Locked { client: owner });
        }
        if state != Dispute::Undisputed {
            return Err(EngineError::AlreadyDisputed { tx });
        }
        let (Some(available), Some(held)) =
            (account.available.checked_sub(amount), account.held.checked_add(amount))
        else {
            return Err(EngineError::Overflow { client: owner, tx });
        };
        account.available = available;
        account.held = held;
        self.set_dispute(tx, Dispute::Open);
        Ok(())
    }

    fn resolve(&mut self, client: ClientId, tx: TxId) -> Result<(), EngineError> {
        let amount = self.disputed_amount(client, tx)?;
        let account = self.accounts.entry(client).or_insert_with(Account::new);
        let (Some(held), Some(available)) =
            (account.held.checked_sub(amount), account.available.checked_add(amount))
        else {
            return Err(EngineError::Overflow { client, tx });
        };
        account.held = held;
        account.available = available;
        self.set_dispute(tx, Dispute::Resolved);
        Ok(())
    }

    fn chargeback(&mut self, client: ClientId, tx: TxId) -> Result<(), EngineError> {
        let amount = self.disputed_amount(client, tx)?;
        let account = self.accounts.entry(client).or_insert_with(Account::new);
        let Some(held) = account.held.checked_sub(amount) else {
            return Err(EngineError::Overflow { client, tx });
        };
        account.held = held;
        account.locked = true;
        self.set_dispute(tx, Dispute::ChargedBack);
        Ok(())
    }

    fn disputed_amount(&self, client: ClientId, tx: TxId) -> Result<Amount, EngineError> {
        let (owner, amount, state) = match self.entries.get(&tx) {
            None => return Err(EngineError::UnknownTx { tx }),
            Some(Entry::Deposit { client, amount, dispute }) => (*client, *amount, *dispute),
            Some(_) => return Err(EngineError::NotUnderDispute { tx }),
        };
        if owner != client {
            return Err(EngineError::ClientMismatch { client, tx });
        }
        if self.accounts.get(&owner).is_some_and(|account| account.locked) {
            return Err(EngineError::Locked { client: owner });
        }
        if state != Dispute::Open {
            return Err(EngineError::NotUnderDispute { tx });
        }
        Ok(amount)
    }

    fn set_dispute(&mut self, tx: TxId, state: Dispute) {
        if let Some(Entry::Deposit { dispute, .. }) = self.entries.get_mut(&tx) {
            *dispute = state;
        }
    }
}

impl Account {
    fn new() -> Self {
        Self { available: Amount::ZERO, held: Amount::ZERO, locked: false }
    }

    pub fn available(&self) -> Amount {
        self.available
    }

    pub fn held(&self) -> Amount {
        self.held
    }

    pub fn total(&self) -> Amount {
        // Saturating is a formality: every change stores available and held only if their sum fits.
        Amount::from_units(self.available.units().saturating_add(self.held.units()))
    }

    pub fn locked(&self) -> bool {
        self.locked
    }
}

fn burn_id(entries: &mut HashMap<TxId, Entry>, tx: TxId) {
    entries.entry(tx).or_insert(Entry::Rejected);
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locked { client } => write!(f, "client {} is locked", client.get()),
            Self::DuplicateTx { tx } => write!(f, "transaction {} already exists", tx.get()),
            Self::InsufficientFunds { client, tx } => {
                write!(
                    f,
                    "client {} has insufficient funds for transaction {}",
                    client.get(),
                    tx.get()
                )
            }
            Self::Overflow { client, tx } => {
                write!(
                    f,
                    "transaction {} overflows the balance of client {}",
                    tx.get(),
                    client.get()
                )
            }
            Self::UnknownTx { tx } => write!(f, "transaction {} is unknown", tx.get()),
            Self::NotDisputable { tx } => write!(f, "transaction {} cannot be disputed", tx.get()),
            Self::NotUnderDispute { tx } => {
                write!(f, "transaction {} is not under dispute", tx.get())
            }
            Self::ClientMismatch { client, tx } => {
                write!(f, "client {} does not own transaction {}", client.get(), tx.get())
            }
            Self::AlreadyDisputed { tx } => {
                write!(f, "transaction {} was already disputed", tx.get())
            }
        }
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests;
