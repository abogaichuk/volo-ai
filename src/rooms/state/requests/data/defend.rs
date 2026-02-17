use std::collections::HashMap;

use screeps::{Creep, INVADER_USERNAME, Part, RoomName, SOURCE_KEEPER_USERNAME, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::commons::find_roles;
use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::BoostReason;
use crate::rooms::state::requests::{Assignment, CreepHostile, Meta, Status};
use crate::units::creeps::CreepMemory;
use crate::units::roles::Role;
use crate::units::roles::combat::defender::Defender;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DefendData {
    pub room_name: RoomName,
    #[serde(default)]
    pub hostiles: Vec<CreepHostile>,
}

impl DefendData {
    pub const fn new(room_name: RoomName) -> Self {
        Self { room_name, hostiles: Vec::new() }
    }

    pub const fn with_hostiles(room_name: RoomName, hostiles: Vec<CreepHostile>) -> Self {
        Self { room_name, hostiles }
    }
}

pub(in crate::rooms::state::requests) fn defend_handler(
    data: &DefendData,
    meta: &mut Meta,
    _assignment: &mut Assignment,
    home: &Shelter,
    creeps: &HashMap<String, CreepMemory>,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    if let Some(farm) = home.get_farm(data.room_name) {
        if meta.created_at + 1500 > game::time() {
            match meta.status {
                //wait 5 ticks to enter entire squad
                Status::Created if meta.updated_at + 5 < game::time() => {
                    let (invanders, players): (Vec<_>, Vec<_>) = farm
                        .hostiles()
                        .iter()
                        .filter(|h| h.owner().username() != SOURCE_KEEPER_USERNAME)
                        .filter(|h| h.body().len() > 2)
                        .partition(|h| h.owner().username() == INVADER_USERNAME);

                    let additional = if players.is_empty() && !invanders.is_empty() {
                        let defender = Role::Defender(Defender::new(Some(home.name()), false));
                        let alive_number = find_roles(&defender, home.spawn_queue(), creeps);

                        match invanders.len() {
                            0..=2 if alive_number == 0 => (defender, 1),
                            3 | 4 if alive_number < 2 => (defender, 2 - alive_number),
                            5.. if alive_number < 3 => (defender, 3 - alive_number),
                            _ => (defender, 0),
                        }
                    } else if !players.is_empty() && invanders.is_empty() {
                        let need_boost = need_boost(&players);

                        events.push(RoomEvent::AddBoost(BoostReason::Defend, 300));
                        let defender = Role::Defender(Defender::new(Some(home.name()), need_boost));
                        let alive_number = find_roles(&defender, home.spawn_queue(), creeps);

                        if alive_number == 0 {
                            (defender, 1)
                        } else {
                            (defender, 0)
                        }
                        // match players.len() {
                        //     0..=2 if alive_number == 0 => (defender, 1),
                        //     _ if alive_number < 2 => (defender, 2 - alive_number),
                        //     _ => (defender, 0),
                        // }
                    } else {
                        (Role::Defender(Defender::new(Some(home.name()), false)), 0)
                    };

                    if additional.1 > 0 {
                        events.push(RoomEvent::Spawn(additional.0, additional.1));
                        meta.update(Status::InProgress);
                    } else {
                        meta.update(Status::Aborted);
                    }
                }
                Status::InProgress if meta.updated_at + 450 < game::time() => {
                    let (invanders, players): (Vec<_>, Vec<_>) = farm
                        .hostiles()
                        .iter()
                        .filter(|h| h.owner().username() != SOURCE_KEEPER_USERNAME)
                        .partition(|h| h.owner().username() == INVADER_USERNAME);

                    let additional = if players.is_empty() && !invanders.is_empty() {
                        let defender = Role::Defender(Defender::new(Some(home.name()), false));
                        let alive_number = find_roles(&defender, home.spawn_queue(), creeps);

                        match invanders.len() {
                            0..=2 if alive_number == 0 => (defender, 2 - alive_number),
                            3 | 4 if alive_number < 2 => (defender, 3 - alive_number),
                            5.. if alive_number < 3 => (defender, 4 - alive_number),
                            _ => (defender, 0),
                        }
                    } else if !players.is_empty() && invanders.is_empty() {
                        let need_boost = need_boost(&players);

                        events.push(RoomEvent::AddBoost(BoostReason::Defend, 300));
                        let defender = Role::Defender(Defender::new(Some(home.name()), need_boost));
                        let alive_number = find_roles(&defender, home.spawn_queue(), creeps);

                        if alive_number == 0 {
                            (defender, 1)
                        } else {
                            (defender, 0)
                        }
                        // match players.len() {
                        //     0..=2 if alive_number == 0 => (defender, 1),
                        //     _ if alive_number < 2 => (defender, 2 - alive_number),
                        //     _ => (defender, 0),
                        // }
                    } else {
                        (Role::Defender(Defender::new(Some(home.name()), false)), 0)
                    };

                    if additional.1 > 0 {
                        events.push(RoomEvent::Spawn(additional.0, additional.1));
                    } else {
                        meta.update(Status::Aborted);
                    }
                }
                _ => {}
            }
        } else {
            meta.update(Status::Aborted);
        }
    }

    events
}

fn need_boost(hostiles: &[&Creep]) -> bool {
    hostiles.iter().any(|h| h.body().iter()
        .any(|body_part| body_part.boost().is_some()))
        || hostiles.iter().any(|h| {
            h.body().iter().any(|bp| bp.part() == Part::Attack || bp.part() == Part::RangedAttack)
        })
}

// fn need_boost(hostiles: &[CreepHostile]) -> bool {
//     hostiles.iter().any(|h| h.owner != INVADER_USERNAME && h.parts.iter().any(|part| part.boosted))
//         || hostiles.iter().filter(|h| h.owner != INVADER_USERNAME).count() > 1
// }
