use std::{
    fmt,
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};
use screeps::{game, Deposit, ObjectId, Position, RoomName};

#[derive(Serialize, Deserialize, Eq)]
pub(crate) struct DepositOrder {
    pub(crate) room: Option<RoomName>,
    pub(crate) id: ObjectId<Deposit>,
    pub(crate) pos: Position,
    pub(crate) cells: usize,
    pub(crate) timeout: u32,
}

impl DepositOrder {
    pub(crate) fn new(id: ObjectId<Deposit>, pos: Position, cells: usize) -> Self {
        Self {
            room: None,
            id,
            pos,
            cells,
            timeout: game::time() + 5000,
        }
    }
}

impl PartialEq for DepositOrder {
    fn eq(&self, other: &DepositOrder) -> bool {
        self.id == other.id
    }
}

impl Hash for DepositOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for DepositOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DepositOrder[base: {:?}, id: {}, pos: {}, cells: {}]",
            self.room, self.id, self.pos, self.cells
        )
    }
}
