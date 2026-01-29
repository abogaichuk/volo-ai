use log::warn;
use screeps::{ObjectId, Position, RoomName, StructureController, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::services::booker::Booker;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookData {
    pub id: ObjectId<StructureController>,
    pub pos: Position,
}

impl BookData {
    pub const fn new(id: ObjectId<StructureController>, pos: Position) -> Self {
        Self { id, pos }
    }
}

pub(in crate::rooms::state::requests) fn book_handler(
    data: &BookData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let booker = Role::Booker(Booker::new(Some(home.name())));
    match meta.status {
        Status::Created if !home.spawn_queue().contains(&booker) => {
            meta.update(Status::Spawning);
            events.push(RoomEvent::Spawn(booker, 1));
            warn!("{} spawned booker for: {:?}", home.name(), data);
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
    }
    events
}
