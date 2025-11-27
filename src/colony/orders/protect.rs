use std::{
    fmt,
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};
use screeps::{game, RoomName};

#[derive(Serialize, Deserialize, Eq)]
pub(crate) struct ProtectOrder {
    pub(crate) room: Option<RoomName>,
    pub(crate) target: RoomName,
    pub(crate) ctrl_lvl: u8,
    pub(crate) timeout: u32,
}

impl ProtectOrder {
    pub(crate) fn new(target: RoomName, ctrl_lvl: u8) -> Self {
        Self {
            room: None,
            target,
            ctrl_lvl,
            timeout: game::time() + 2000,
        }
    }
}

impl PartialEq for ProtectOrder {
    fn eq(&self, other: &ProtectOrder) -> bool {
        self.target == other.target
    }
}

impl Hash for ProtectOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.target.hash(state);
    }
}

impl fmt::Debug for ProtectOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProtectOrder[base: {:?}, target: {}, ctrl_lvl: {}]",
            self.room, self.target, self.ctrl_lvl
        )
    }
}
