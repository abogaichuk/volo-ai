use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, Structure, Position, RoomName};
use smallvec::SmallVec;
use crate::{
    units::roles::{Role, services::{dismantler::Dismantler, puller::Puller}},
    rooms::{RoomEvent, state::requests::{Meta, Status, Assignment}},
    commons::is_walkable
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DismantleData {
    pub id: ObjectId<Structure>,
    pub workplace: Position,
}

impl DismantleData {
    pub fn new(id: ObjectId<Structure>, workplace: Position) -> Self {
        Self { id, workplace }
    }
}

pub(in crate::rooms::state::requests) fn dismantle_handler(
    data: &mut DismantleData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let dismantler = Role::Dismantler(Dismantler::new(Some(home_name)));
    let puller = Role::Puller(Puller::new(Some(home_name)));
    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);

            events.push(RoomEvent::Spawn(dismantler, 1));
            events.push(RoomEvent::Spawn(puller, 1));
        },
        Status::InProgress if game::time() % 100 == 0 && !assignment.has_alive_members() => {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        Status::OnHold => {
            if is_walkable(&data.workplace) {
                meta.update(Status::Created);
            }
        },
        _ => {}
    }
    events
}