use serde::{Serialize, Deserialize};
use screeps::{game, RoomName};
use smallvec::SmallVec;
use std::collections::HashMap;
use crate::{
    rooms::{RoomEvent, shelter::Shelter, state::requests::{Assignment, CreepHostile, Meta, Status}},
    units::{Memory, roles::{Role, combat::defender::Defender}},
    commons::find_roles
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DefendData {
    pub room_name: RoomName,
    #[serde(default)]
    pub hostiles: Vec<CreepHostile>,
}

impl DefendData {
    pub fn new(room_name: RoomName) -> Self {
        Self { room_name, hostiles: Vec::new() }
    }

    pub fn with_hostiles(room_name: RoomName, hostiles: Vec<CreepHostile>) -> Self {
        Self { room_name, hostiles }
    }
}

pub(in crate::rooms::state::requests) fn defend_handler(
    data: &DefendData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home: &Shelter,
    creeps: &HashMap<String, Memory>
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    if meta.created_at + 1500 > game::time() {
        match meta.status {
            Status::Created => {
                meta.update(Status::InProgress);

                let defender = Role::Defender(Defender::new(Some(home.name())));
                let alive_number = find_roles(&defender, home.spawn_queue(), creeps);

                let additional = match data.hostiles.len() {
                    0..=2 if alive_number == 0 => 1,
                    3 | 4 if alive_number < 2 => 2 - alive_number,
                    5.. if alive_number < 3 => 3 - alive_number,
                    _ => 0
                };

                if additional > 0 {
                    events.push(RoomEvent::Spawn(defender, additional));
                }
            }, 
            Status::InProgress if meta.updated_at + 450 < game::time() => {
                let defender = Role::Defender(Defender::new(Some(home.name())));
                let alive_number = find_roles(&defender, home.spawn_queue(), creeps);

                let additional = match data.hostiles.len() {
                    0..=2 if alive_number == 0 => 2 - alive_number,
                    3 | 4 if alive_number < 2 => 3 - alive_number,
                    5.. if alive_number < 3 => 4 - alive_number,
                    _ => 0
                };

                if additional > 0 {
                    events.push(RoomEvent::Spawn(defender, additional));
                }
            },
            _ => {}
        };
    } else {
        meta.update(Status::Aborted);
    }
    events
}