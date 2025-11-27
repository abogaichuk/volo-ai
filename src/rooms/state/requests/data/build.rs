use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, ConstructionSite, Position};
use smallvec::{smallvec, SmallVec};
use crate::rooms::{RoomEvent, state::requests::{Meta, Status, Assignment}};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildData {
    pub id: Option<ObjectId<ConstructionSite>>,
    pub pos: Position,
}

impl BuildData {
    pub fn new(id: Option<ObjectId<ConstructionSite>>, pos: Position) -> Self {
        Self { id, pos }
    }
}

pub(in crate::rooms::state::requests) fn build_handler(
    meta: &mut Meta,
    assignment: &mut Assignment
) -> SmallVec<[RoomEvent; 3]> {
    match meta.status {
        Status::InProgress if game::time() % 100 == 0 && !assignment.has_alive_members() => {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    };
    smallvec![]
}