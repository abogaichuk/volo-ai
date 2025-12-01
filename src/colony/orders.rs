mod caravan;
mod deposit;
mod powerbank;
mod protect;
mod resource;
mod withdraw;

pub(crate) use caravan::CaravanOrder;
pub(crate) use deposit::DepositOrder;
pub(crate) use powerbank::PowerbankOrder;
pub(crate) use protect::ProtectOrder;
pub(crate) use resource::ResourceOrder;
pub(crate) use withdraw::WithdrawOrder;

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

//todo different flows for send boost resources forced or not(for instance when base is attacked or not)
// initially check for excess resources - if not for compressed resources if not - craft by yourself, if need urgently - send what it has
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum ColonyOrder {
    Powerbank(PowerbankOrder),
    Deposit(DepositOrder),
    Withdraw(WithdrawOrder),
    Protect(ProtectOrder),
    Caravan(CaravanOrder),
    Excess(ResourceOrder),
    Lack(ResourceOrder),
}

impl ColonyOrder {
    pub(crate) fn timeout(&self) -> u32 {
        match self {
            ColonyOrder::Powerbank(p) => p.timeout,
            ColonyOrder::Deposit(d) => d.timeout,
            ColonyOrder::Withdraw(w) => w.timeout,
            ColonyOrder::Protect(p) => p.timeout,
            ColonyOrder::Caravan(c) => c.timeout,
            ColonyOrder::Excess(e) => e.timeout,
            ColonyOrder::Lack(l) => l.timeout,
        }
    }
}

impl Hash for ColonyOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ColonyOrder::Powerbank(p) => p.hash(state),
            ColonyOrder::Deposit(d) => d.hash(state),
            ColonyOrder::Withdraw(w) => w.hash(state),
            ColonyOrder::Protect(p) => p.hash(state),
            ColonyOrder::Caravan(c) => c.hash(state),
            ColonyOrder::Excess(e) => e.hash(state),
            ColonyOrder::Lack(l) => l.hash(state),
        }
    }
}

impl Eq for ColonyOrder {}
impl PartialEq for ColonyOrder {
    fn eq(&self, other: &ColonyOrder) -> bool {
        match self {
            ColonyOrder::Powerbank(pb) => matches!(other, ColonyOrder::Powerbank(o) if pb == o),
            ColonyOrder::Deposit(d) => matches!(other, ColonyOrder::Deposit(o) if d == o),
            ColonyOrder::Withdraw(w) => matches!(other, ColonyOrder::Withdraw(o) if w == o),
            ColonyOrder::Protect(p) => matches!(other, ColonyOrder::Protect(o) if p == o),
            ColonyOrder::Caravan(c) => matches!(other, ColonyOrder::Caravan(o) if c == o),
            ColonyOrder::Excess(e) => matches!(other, ColonyOrder::Excess(o) if e == o),
            ColonyOrder::Lack(l) => matches!(other, ColonyOrder::Lack(o) if l == o),
        }
    }
}
