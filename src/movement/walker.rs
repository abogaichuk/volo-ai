use screeps::{game, Creep, Position, HasPosition, HasHits, RoomXY, look::STRUCTURES, StructureObject, Part, RoomName};
use std::collections::HashSet;
use crate::{
    units::roles::{Role, Kind},
    commons::{closest_creep, try_heal, in_range_to, has_part, get_positions_near_by, find_flags},
    utils::constants::LONG_RANGE_ACTION
};
use super::{MovementGoalBuilder, MovementGoal, MovementProfile};


#[derive(Debug, Clone)]
pub enum Walker {
    Berserk,
    Aggressive,
    Therapeutic,
    Reinforcing,
    //todo EnergyFinding
    Exploring(bool), //avoid my creeps or not
    Flee,
    Immobile
}

impl Walker {
    pub fn walk(&self, to: Position, range: u32, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> MovementGoal {
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
                try_heal(creep);

                if let Some(hostile) = closest_creep(creep, enemies.iter()) {
                    if has_part(&[Part::Attack], creep, true) &&
                        creep.pos().is_near_to(hostile.pos())
                    {
                        let _ = creep.attack(hostile);
                        let _ = creep.say("🖕", true); //finger
                    } else if in_range_to(creep, enemies.iter(), LONG_RANGE_ACTION) > 2 {
                        let _ = creep.ranged_mass_attack();
                        let _ = creep.say("🖕", true); //finger
                    } else {
                        let _ = creep.ranged_attack(hostile);
                    }
                }

                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    .build()
            },
            Self::Therapeutic => {
                try_heal(creep);

                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    // .danger_zones(get_danger_zones(&hostiles)) //quick fix stuck when in danger zones
                    .build()
            }
            Self::Reinforcing => {
                if let Ok(structures) = creep.pos().look_for(STRUCTURES) {
                    let _ = structures.iter()
                        .find_map(|structure| {
                            match structure {
                                StructureObject::StructureRoad(road) if road.hits() < (road.hits_max() as f32 / 1.5) as u32 => Some(road),
                                _ => None
                            }
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
            Self::Flee => {
                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    .avoid_creeps(true)
                    .flee()
                    .build()
            }
            Self::Immobile => {
                MovementGoalBuilder::new(to)
                    .range(range)
                    .profile(role.get_movement_profile(creep))
                    .build()
            }
        }
    }
}

fn get_danger_zones(room_name: RoomName, enemies: &[Creep]) -> Option<(RoomName, Vec<RoomXY>)> {
    let mut cells_under_attack:Vec<RoomXY> = enemies.iter()
        .filter_map(|enemy| {
            if has_part(&[Part::RangedAttack], enemy, true) {
                Some(get_positions_near_by(enemy.pos(), 3, true, false)
                    .into_iter()
                    .map(|(x, y)| unsafe { RoomXY::unchecked_new(x, y) })
                    .collect::<HashSet<RoomXY>>()
                )
            } else if has_part(&[Part::Attack], enemy, true) {
                Some(get_positions_near_by(enemy.pos(), 1, true, false)
                    .into_iter()
                    .map(|(x, y)| unsafe { RoomXY::unchecked_new(x, y) })
                    .collect::<HashSet<RoomXY>>()
                )
            } else {
                None
            }
        })
        .flatten()
        .collect();

    cells_under_attack.extend(flags_as_danger_zones(&room_name));

    if cells_under_attack.is_empty() {
        None
    } else {
        Some((room_name, cells_under_attack))
    }
}

fn flags_as_danger_zones(&room_name: &RoomName) -> HashSet<RoomXY> {
    let room = game::rooms().get(room_name).expect("expect room");
    find_flags(&room).iter()
        .flat_map(|flag| {
            get_positions_near_by(flag.pos(), 3, true, false)
                    .into_iter()
                    .map(|(x, y)| unsafe { RoomXY::unchecked_new(x, y) })
                    .collect::<HashSet<RoomXY>>()
        })
        .collect()
}