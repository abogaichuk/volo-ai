use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, StructureInvaderCore, Position};
use smallvec::SmallVec;
use std::collections::HashMap;
use crate::{
    rooms::{Shelter, RoomEvent, state::requests::{Assignment, Meta, Status}},
    units::{creeps::CreepMemory, roles::{Role, combat::overseer::Overseer}},
    commons::find_roles
};

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
    creeps: &HashMap<String, CreepMemory>
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    match meta.status {
        Status::Created => {
            meta.update(Status::Spawning);
            let overseer = Role::Overseer(Overseer::new(None, Some(home.name())));
            if find_roles(&overseer, home.spawn_queue(), creeps) == 0 {
                events.push(RoomEvent::Spawn(overseer, 1));
            }
        },
        Status::InProgress if game::time() % 100 == 0 && !assignment.has_alive_members() => {
            meta.update(Status::Created);
            *assignment = Assignment::Single(None);
        }
        _ => {}
    };
    events
}