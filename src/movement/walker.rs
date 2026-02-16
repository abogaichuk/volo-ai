use std::collections::HashSet;

use log::info;
use screeps::look::STRUCTURES;
use screeps::{
    Creep, HasHits, HasPosition, Part, Position, RoomName, RoomXY, SharedCreepProperties,
    StructureObject, game,
};

use super::{MovementGoal, MovementGoalBuilder, MovementProfile};
use crate::commons::{
    closest_creep, find_flags, get_positions_near_by, has_part, in_range_to, try_heal,
};
use crate::units::roles::{Kind, Role};
use crate::utils::constants::LONG_RANGE_ACTION;

#[derive(Debug, Clone)]
pub enum Walker {
    Berserk,
    //todo improve combat logic
    Aggressive,
    Therapeutic,
    Reinforcing,
    //todo EnergyFinding
    Exploring(bool), //avoid my creeps or not
    Flee,
    Immobile,
}

impl Walker {
    pub fn walk(
        &self,
        to: Position,
        range: u32,
        creep: &Creep,
        role: &Role,
        enemies: Vec<Creep>,
    ) -> MovementGoal {
        match self {
            Self::Berserk => {
                try_heal(creep);
                let _ = creep.ranged_mass_attack();
                let _ = creep.say("💀", true); //skull

                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    .build()
            }
            Self::Aggressive => {
                if let Some(hostile) = closest_creep(creep, enemies.iter()) {
                    if has_part(&[Part::Attack], creep, true)
                        && creep.pos().is_near_to(hostile.pos())
                    {
                        let _ = creep.attack(hostile);
                        let _ = creep.say("🖕", true); //finger
                    } else if in_range_to(creep, enemies.iter(), LONG_RANGE_ACTION) > 2 {
                        let _ = creep.ranged_mass_attack();
                        try_heal(creep);
                    } else {
                        try_heal(creep);
                        let _ = creep.ranged_attack(hostile);
                    }
                }

                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    .build()
            }
            Self::Therapeutic => {
                try_heal(creep);

                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    // .danger_zones(get_danger_zones(&hostiles)) //quick fix stuck when in danger
                    // zones
                    .build()
            }
            Self::Reinforcing => {
                if let Ok(structures) = creep.pos().look_for(STRUCTURES) {
                    let _ = structures
                        .iter()
                        .find_map(|structure| match structure {
                            StructureObject::StructureRoad(road)
                                if road.hits() * 3 < road.hits_max() * 2 =>
                            {
                                Some(road)
                            }
                            _ => None,
                        })
                        .and_then(|road| creep.repair(road).ok());
                }

                let danger_zones = get_danger_zones(creep.pos().room_name(), &enemies);
                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(MovementProfile::RoadsOneToTwo)
                    .avoid_creeps(danger_zones.is_some())
                    .danger_zones(danger_zones)
                    .build()
            }
            Self::Exploring(avoid_creeps) => {
                let danger_zones = get_danger_zones(creep.pos().room_name(), &enemies);
                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    .avoid_creeps(danger_zones.is_some() || *avoid_creeps)
                    .danger_zones(danger_zones)
                    .build()
            }
            Self::Flee => MovementGoalBuilder::new(to)
                .range(range)
                .profile(role.get_movement_profile(creep))
                .avoid_creeps(true)
                .flee()
                .build(),
            Self::Immobile => MovementGoalBuilder::new(to)
                .range(range)
                .profile(role.get_movement_profile(creep))
                .build(),
        }
    }
}

fn get_danger_zones(room_name: RoomName, enemies: &[Creep]) -> Option<(RoomName, Vec<RoomXY>)> {
    let mut cells_under_attack: Vec<RoomXY> = enemies
        .iter()
        .filter_map(|enemy| {
            if has_part(&[Part::RangedAttack], enemy, true) {
                Some(
                    get_positions_near_by(enemy.pos(), 3, true, false)
                        .into_iter()
                        .map(|(x, y)| unsafe { RoomXY::unchecked_new(x, y) })
                        .collect::<HashSet<RoomXY>>(),
                )
            } else if has_part(&[Part::Attack], enemy, true) {
                Some(
                    get_positions_near_by(enemy.pos(), 1, true, false)
                        .into_iter()
                        .map(|(x, y)| unsafe { RoomXY::unchecked_new(x, y) })
                        .collect::<HashSet<RoomXY>>(),
                )
            } else {
                None
            }
        })
        .flatten()
        .collect();

    cells_under_attack.extend(flags_as_danger_zones(room_name));

    if cells_under_attack.is_empty() { None } else { Some((room_name, cells_under_attack)) }
}

fn flags_as_danger_zones(room_name: RoomName) -> HashSet<RoomXY> {
    let room = game::rooms().get(room_name).expect("expect room");
    find_flags(&room)
        .iter()
        .flat_map(|flag| {
            get_positions_near_by(flag.pos(), 3, true, false)
                .into_iter()
                .map(|(x, y)| unsafe { RoomXY::unchecked_new(x, y) })
                .collect::<HashSet<RoomXY>>()
        })
        .collect()
}
