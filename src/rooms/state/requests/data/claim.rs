use screeps::{ObjectId, Position, RoomName, StructureController, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::services::conqueror::Conqueror;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaimData {
    pub id: ObjectId<StructureController>,
    pub pos: Position,
}

impl ClaimData {
    pub fn new(id: ObjectId<StructureController>, pos: Position) -> Self {
        Self { id, pos }
    }
}

pub(in crate::rooms::state::requests) fn claim_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);
            let conquer = Role::Conqueror(Conqueror::new(Some(home_name)));
            events.push(RoomEvent::Spawn(conquer, 1));
        }
        Status::InProgress | Status::Spawning => {
            if game::time() > meta.updated_at + 600 {
                meta.update(Status::Created);
                *assignment = Assignment::Single(None);
            }
        }
        _ => {}
    };
    events
}
