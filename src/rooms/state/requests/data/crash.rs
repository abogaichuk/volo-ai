use std::collections::HashMap;

use screeps::{ObjectId, Position, StructureInvaderCore, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::commons::find_roles;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::rooms::{RoomEvent, Shelter};
use crate::units::creeps::CreepMemory;
use crate::units::roles::Role;
use crate::units::roles::combat::overseer::Overseer;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrashData {
    pub id: ObjectId<StructureInvaderCore>,
    pub pos: Position,
}

impl CrashData {
    pub fn new(id: ObjectId<StructureInvaderCore>, pos: Position) -> Self {
        Self { id, pos }
    }
}

pub(in crate::rooms::state::requests) fn crash_handler(
    meta: &mut Meta,
    assignment: &mut Assignment,
    home: &Shelter,
    creeps: &HashMap<String, CreepMemory>,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);
            let overseer = Role::Overseer(Overseer::new(None, Some(home.name())));
            if find_roles(&overseer, home.spawn_queue(), creeps) == 0 {
                events.push(RoomEvent::Spawn(overseer, 1));
            }
        }
        Status::InProgress if game::time().is_multiple_of(100) && !assignment.has_alive_members() => {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    };
    events
}
