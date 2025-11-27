use serde::{Serialize, Deserialize};
use screeps::{Position};
use smallvec::{smallvec, SmallVec};
use crate::rooms::RoomEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PullData {
    pub creep_name: String,
    pub destination: Position,
}

impl PullData {
    pub fn new(creep_name: String, destination: Position) -> Self {
        Self { creep_name, destination }
    }
}

pub(in crate::rooms::state::requests) fn pull_handler() -> SmallVec<[RoomEvent; 3]> {
    smallvec![]
}