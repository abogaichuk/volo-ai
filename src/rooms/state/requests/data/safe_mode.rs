use screeps::{ObjectId, Position, RawObjectId, RoomName, StructureController, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::haulers::carrier::Carrier;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SMData {
    pub id: ObjectId<StructureController>,
    pub pos: Position,
    pub storage_id: RawObjectId,
}

impl SMData {
    pub const fn new(
        id: ObjectId<StructureController>,
        pos: Position,
        storage_id: RawObjectId,
    ) -> Self {
        Self { id, pos, storage_id }
    }
}

pub(in crate::rooms::state::requests) fn sm_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);
            let carrier = Role::Carrier(Carrier::new(Some(home_name)));
            events.push(RoomEvent::Spawn(carrier, 1));
        }
        Status::InProgress
            if game::time().is_multiple_of(100) && !assignment.has_alive_members() =>
        {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    }
    events
}
