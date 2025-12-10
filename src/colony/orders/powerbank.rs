use std::fmt;
use std::hash::{Hash, Hasher};

use screeps::{ObjectId, Position, RoomName, StructurePowerBank, game};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq)]
pub(crate) struct PowerbankOrder {
    pub(crate) room: Option<RoomName>,
    pub(crate) id: ObjectId<StructurePowerBank>,
    pub(crate) pos: Position,
    pub(crate) amount: u32,
    pub(crate) timeout: u32,
}

impl PowerbankOrder {
    pub(crate) fn new(id: ObjectId<StructurePowerBank>, pos: Position, amount: u32) -> Self {
        Self { room: None, id, pos, amount, timeout: game::time() + 5000 }
    }
}

impl PartialEq for PowerbankOrder {
    fn eq(&self, other: &PowerbankOrder) -> bool {
        self.id == other.id
    }
}

impl Hash for PowerbankOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for PowerbankOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PowerbankOrder[base: {:?}, id: {}, pos: {}, amount: {}]",
            self.room, self.id, self.pos, self.amount
        )
    }
}
