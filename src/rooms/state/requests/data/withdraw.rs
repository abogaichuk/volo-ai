use screeps::{Position, RawObjectId, ResourceType, game};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawData {
    pub id: RawObjectId,
    pub pos: Position,
    pub resources: Vec<(ResourceType, Option<u32>)>,
}

impl WithdrawData {
    pub fn new(
        id: RawObjectId,
        pos: Position,
        resources: Vec<(ResourceType, Option<u32>)>,
    ) -> Self {
        Self { id, pos, resources }
    }
}

pub(in crate::rooms::state::requests) fn withdraw_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
) -> SmallVec<[RoomEvent; 3]> {
    if meta.created_at + 300 > game::time() {
        match meta.status {
            Status::InProgress if game::time() % 100 == 0 && !assignment.has_alive_members() => {
                meta.update(Status::Created);
                *assignment = Assignment::Single(None);
            }
            _ => {}
        }
    } else {
        meta.update(Status::Aborted);
    };
    smallvec![]
}
