use screeps::{Position, RawObjectId, ResourceType, RoomName, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::haulers::carrier::Carrier;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LRWData {
    pub id: RawObjectId,
    pub pos: Position,
    pub resource: ResourceType,
    pub amount: u32,
}

impl LRWData {
    pub const fn new(id: RawObjectId, pos: Position, resource: ResourceType, amount: u32) -> Self {
        Self { id, pos, resource, amount }
    }
}

pub(in crate::rooms::state::requests) fn lrw_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    if meta.created_at + 5000 > game::time() {
        match meta.status {
            Status::Created => {
                let carrier = Role::Carrier(Carrier::new(Some(home_name)));

                meta.update(Status::Spawning);
                events.push(RoomEvent::Spawn(carrier, 1));
            }
            Status::InProgress if game::time().is_multiple_of(100) && !assignment.has_alive_members() => {
                meta.update(Status::Created);
                *assignment = Assignment::Single(None);
            }
            Status::Spawning if meta.updated_at + 1500 < game::time() => {
                meta.update(Status::Created);
                *assignment = Assignment::Single(None);
            }
            _ => {}
        }
    }
    events
}
