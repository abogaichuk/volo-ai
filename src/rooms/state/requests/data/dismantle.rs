use screeps::{ObjectId, Position, RoomName, Structure, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::commons::is_walkable;
use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::services::dismantler::Dismantler;
use crate::units::roles::services::puller::Puller;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DismantleData {
    pub id: ObjectId<Structure>,
    pub workplace: Position,
}

impl DismantleData {
    pub const fn new(id: ObjectId<Structure>, workplace: Position) -> Self {
        Self { id, workplace }
    }
}

pub(in crate::rooms::state::requests) fn dismantle_handler(
    data: &mut DismantleData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let dismantler = Role::Dismantler(Dismantler::new(Some(home_name)));
    let puller = Role::Puller(Puller::new(Some(home_name)));
    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);

            events.push(RoomEvent::Spawn(dismantler, 1));
            events.push(RoomEvent::Spawn(puller, 1));
        }
        Status::InProgress if game::time().is_multiple_of(100) && !assignment.has_alive_members() => {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        Status::OnHold => {
            if is_walkable(&data.workplace) {
                meta.update(Status::Created);
            }
        }
        _ => {}
    }
    events
}
