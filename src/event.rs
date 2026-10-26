//! The typed events and ids that cross from parsing into the engine.

use crate::Amount;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientId(u16);

impl ClientId {
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TxId(u32);

impl TxId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Deposit { client: ClientId, tx: TxId, amount: Amount },
    Withdrawal { client: ClientId, tx: TxId, amount: Amount },
    Dispute { client: ClientId, tx: TxId },
    Resolve { client: ClientId, tx: TxId },
    Chargeback { client: ClientId, tx: TxId },
}
