use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, StructureController, Position, RawObjectId, RoomName};
use smallvec::SmallVec;
use crate::{
    rooms::{RoomEvent, state::requests::{Meta, Status, Assignment}},
    units::roles::{Role, haulers::carrier::Carrier}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SMData {
    pub id: ObjectId<StructureController>,
    pub pos: Position,
    pub storage_id: RawObjectId,
}

impl SMData {
    pub fn new(id: ObjectId<StructureController>, pos: Position, storage_id: RawObjectId) -> Self {
        Self { id, pos, storage_id }
    }
}

pub(in crate::rooms::state::requests) fn sm_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);
            let carrier = Role::Carrier(Carrier::new(Some(home_name)));
            events.push(RoomEvent::Spawn(carrier, 1));
        }
        Status::InProgress if game::time() % 100 == 0 && !assignment.has_alive_members() => {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    };
    events
}