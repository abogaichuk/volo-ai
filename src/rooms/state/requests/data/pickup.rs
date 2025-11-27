use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, Resource};
use smallvec::{smallvec, SmallVec};
use crate::rooms::{RoomEvent, state::requests::{Meta, Status}};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PickupData {
    pub id: ObjectId<Resource>,
}

impl PickupData {
    pub fn new(id: ObjectId<Resource>) -> Self {
        Self { id }
    }
}

pub(in crate::rooms::state::requests) fn pickup_handler(meta: &mut Meta) -> SmallVec<[RoomEvent; 3]> {
    if meta.created_at + 300 > game::time() {
    } else {
        meta.update(Status::Aborted);
    };
    smallvec![]
}