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

enum Dispute {
    None,
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

const _: () = assert!(std::mem::size_of::<Entry>() <= 16);

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self { accounts: BTreeMap::new(), entries: HashMap::new() }
    }

    pub fn apply(&mut self, _event: Event) -> Result<(), EngineError> {
        Ok(())
    }

    pub fn burn(&mut self, _tx: TxId) {}

    pub fn accounts(&self) -> impl Iterator<Item = (ClientId, &Account)> {
        std::iter::empty()
    }
}

impl Account {
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
