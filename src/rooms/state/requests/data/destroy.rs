use screeps::Position;
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::RoomEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DestroyData {
    pub target: Position,
    pub level: u8,
}

pub(in crate::rooms::state::requests) fn destroy_handler() -> SmallVec<[RoomEvent; 3]> {
    smallvec![]
}
