use screeps::{ConstructionSite, ObjectId, Position, game};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildData {
    pub id: Option<ObjectId<ConstructionSite>>,
    pub pos: Position,
}

impl BuildData {
    pub const fn new(id: Option<ObjectId<ConstructionSite>>, pos: Position) -> Self {
        Self { id, pos }
    }
}

pub(in crate::rooms::state::requests) fn build_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
) -> SmallVec<[RoomEvent; 3]> {
    match meta.status {
        Status::InProgress
            if game::time().is_multiple_of(100) && !assignment.has_alive_members() =>
        {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    }
    smallvec![]
}
