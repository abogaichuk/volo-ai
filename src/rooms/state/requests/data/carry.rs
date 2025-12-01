use std::fmt::{Display, Formatter};

use serde::{Serialize, Deserialize};
use screeps::{game, RawObjectId, ResourceType};
use smallvec::{smallvec, SmallVec};
use crate::rooms::{RoomEvent, state::requests::{Meta, Status, Assignment}};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarryData {
    pub from: RawObjectId,
    pub to: RawObjectId,
    pub resource: ResourceType,
    pub amount: u32,
}

impl CarryData {
    pub fn new(from: RawObjectId, to: RawObjectId, resource: ResourceType, amount: u32) -> Self {
        Self { from, to, resource, amount }
    }
}

impl Display for CarryData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "res: {}, amount: {}, from: {}, to: {}", self.resource, self.amount, self.from, self.to)
    }
}

pub(in crate::rooms::state::requests) fn carry_handler(
    meta: &mut Meta,
    assignment: &mut Assignment
) -> SmallVec<[RoomEvent; 3]> {
    match meta.status {
        Status::InProgress if game::time() % 100 == 0 && !assignment.has_alive_members() => {
                meta.update(Status::Created);
                *assignment = Assignment::Single(None);
            }
        _ => {}
    }
    smallvec![]
}