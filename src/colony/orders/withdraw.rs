use std::fmt;
use std::hash::{Hash, Hasher};

use screeps::{Position, RawObjectId, ResourceType, RoomName, game};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq)]
pub(crate) struct WithdrawOrder {
    pub(crate) room: Option<RoomName>,
    pub(crate) id: RawObjectId,
    pub(crate) pos: Position,
    pub(crate) resource: ResourceType,
    pub(crate) amount: u32,
    pub(crate) timeout: u32,
}

impl WithdrawOrder {
    pub(crate) fn new(id: RawObjectId, pos: Position, resource: ResourceType, amount: u32) -> Self {
        Self { room: None, id, pos, resource, amount, timeout: game::time() + 2000 }
    }
}

impl PartialEq for WithdrawOrder {
    fn eq(&self, other: &WithdrawOrder) -> bool {
        self.id == other.id
    }
}

impl Hash for WithdrawOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for WithdrawOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WithdrawOrder[base: {:?}, id: {}, pos: {}, resource: {}, amount: {}]",
            self.room, self.id, self.pos, self.resource, self.amount
        )
    }
}
