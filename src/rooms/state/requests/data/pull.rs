use screeps::Position;
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::RoomEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PullData {
    pub creep_name: String,
    pub destination: Position,
}

impl PullData {
    pub const fn new(creep_name: String, destination: Position) -> Self {
        Self { creep_name, destination }
    }
}

pub(in crate::rooms::state::requests) fn pull_handler() -> SmallVec<[RoomEvent; 3]> {
    smallvec![]
}
