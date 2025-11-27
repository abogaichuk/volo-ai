use serde::{Serialize, Deserialize};
use screeps::{Position};
use smallvec::{smallvec, SmallVec};
use crate::rooms::RoomEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DestroyData {
    pub target: Position,
    pub level: u8,
}

pub(in crate::rooms::state::requests) fn destroy_handler() -> SmallVec<[RoomEvent; 3]> {
    smallvec![]
}