use log::warn;
use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, StructureController, Position, RoomName};
use smallvec::SmallVec;
use crate::{
    units::roles::{Role, services::booker::Booker},
    rooms::{RoomEvent, state::requests::{Meta, Status, Assignment}}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookData {
    pub id: ObjectId<StructureController>,
    pub pos: Position,
}

impl BookData {
    pub fn new(id: ObjectId<StructureController>, pos: Position) -> Self {
        Self { id, pos }
    }
}

pub(in crate::rooms::state::requests) fn book_handler(
    data: &BookData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);
            let booker = Role::Booker(Booker::new(Some(home_name)));
            events.push(RoomEvent::Spawn(booker, 1));
            warn!("{} spawned booker for: {:?}", home_name, data);
        }
        Status::InProgress if !assignment.has_alive_members() => {
            meta.update(Status::Aborted);
            *assignment = Assignment::Single(None);
        }
        Status::Spawning if game::time() > meta.updated_at + 600 => {
            meta.update(Status::Aborted);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    };
    events
}