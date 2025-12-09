use screeps::{ObjectId, Position, Structure, game};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepairData {
    pub id: ObjectId<Structure>,
    pub pos: Position,
    pub hits: u32,
    pub attempts: u8,
    pub attempts_max: u8,
}

impl RepairData {
    pub const fn new(id: ObjectId<Structure>, pos: Position, attempts: u8) -> Self {
        Self { id, pos, hits: 100, attempts, attempts_max: 5 }
    }

    pub const fn with_max_attempts_and_hits(
        id: ObjectId<Structure>,
        pos: Position,
        attempts_max: u8,
        hits: u32,
    ) -> Self {
        Self { id, pos, hits, attempts: 0, attempts_max }
    }
}

pub(in crate::rooms::state::requests) fn repair_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
) -> SmallVec<[RoomEvent; 3]> {
    if meta.created_at + 1500 > game::time() {
        match meta.status {
            Status::InProgress if game::time().is_multiple_of(100) && !assignment.has_alive_members() => {
                meta.update(Status::Created);
                *assignment = Assignment::Single(None);
            }
            _ => {}
        }
    } else {
        meta.update(Status::Resolved);
    }
    smallvec![]
}
