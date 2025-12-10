use screeps::{ObjectId, Resource, game};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Meta, Status};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PickupData {
    pub id: ObjectId<Resource>,
}

impl PickupData {
    pub const fn new(id: ObjectId<Resource>) -> Self {
        Self { id }
    }
}

pub(in crate::rooms::state::requests) fn pickup_handler(
    meta: &mut Meta,
) -> SmallVec<[RoomEvent; 3]> {
    if meta.created_at + 300 > game::time() {
    } else {
        meta.update(Status::Aborted);
    }
    smallvec![]
}
