use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{Part, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, can_scale, default_parts_priority};
use crate::movement::MovementProfile;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DismantlerWithHeal {
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for DismantlerWithHeal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl DismantlerWithHeal {
    pub const fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}
impl Kind for DismantlerWithHeal {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Work, Part::Heal, Part::Move, Part::Move];

        let mut body = [Part::Work, Part::Move].into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() > creep.hits_max() - creep.hits_max() / 5 {
            MovementProfile::PlainsOneToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }
}

// pub fn run(creep: &Creep, memory: &mut CreepMemory, _: RoomName, _: &mut
// RoomMemory) -> Option<MovementGoal> {     let room =
// creep.room().expect("couldn't resolve a room");

//     if let Some(target) = memory.target.as_ref() {

//         if is_wounded(creep) {
//             let _ = creep.heal(creep);
//             if room.name() == target.room_name {
//                 //if in a target room - go to exit
//                 info!("injured in target room");
//                 move_to_exit(creep);
//             } else {
//                 //if on edge move 1 step to the room center for heal
//                 info!("injured in another room");
//                 if creep.pos().is_room_edge() {
//                     if let Some(direction) = get_direction_into(creep.pos())
// {                         let _ = creep.move_direction(direction);
//                     }
//                 } else {
//                     //just wait
//                 }
//             }
//         } else if creep.hits() == creep.hits_max() {
//             if room.name() == target.room_name {
//                 let target_position = RoomPosition::new(target.x, target.y,
// target.room_name).into();

//                 if creep.pos().is_equal_to(target_position) {
//                     //todo refactoring change string to target:
// ObjectId<Structure>,                     let structure =
// ObjectId::<Structure>::from_str(&target.id).ok()
// .map_or_else(|| None, |o_id| o_id.resolve());                     info!("{}
// structure to be dismantled: {:?}", creep.name(), structure);

//                     if let Some(structure) = structure {
//                         match
// StructureObject::from(structure).as_dismantleable() {
// Some(dismantleable) => match creep.dismantle(dismantleable) {
// Ok(()) => {info!("{} dismantle OK!", creep.name())},
// Err(e) => warn!("{} dismantle error: {:?}", creep.name(), e)
// }                             None => {
//                                 warn!("{} isn't dismantleable structure:
// {:?}", creep.name(), target.id);                             }
//                         }
//                     } else {
//                         //if no structure wait while tower is on low energy
//                         if let Some(tower) =
// commons::find_hostile_tower(&room) {                             if
// tower.store().get_used_capacity(Some(ResourceType::Energy)) > 50 {
//                                 //wait near the room border
//                                 if !commons::is_near_edge(creep.pos()) {
//                                     move_to_exit(creep);
//                                 }
//                             } else if creep.pos().is_near_to(tower.pos()) {
//                                 let _ = creep.dismantle(&tower);
//                             } else {
//                                 let _ = creep.move_to(tower);
//                             }
//                         } else {
//                             //todo destroy spawns
//                             info!("no towers in the room: {}", room.name())
//                         }
//                     }
//                 } else {
//                     let _ = creep.move_to(target_position);
//                 }
//             } else {
//                 let target_position:Position = RoomPosition::new(target.x,
// target.y, target.room_name).into();                 let _ =
// creep.move_to(target_position);             }
//         } else {
//             let _ = creep.heal(creep);
//             //just wait??
//         }
//     }
//     None
// }

// fn get_direction_into(position: Position) -> Option<Direction> {
//     commons::find_walkable_positions_near_by(position, true)
//         .first()
//         .and_then(|to_position| position.get_direction_to(*to_position))
// }

// fn is_wounded(creep: &Creep) -> bool {
//     // creep.hits() <= creep.hits_max() - creep.hits_max() / 10 // <= than
// 90% == 5 parts     creep.hits() <= creep.hits_max() - creep.hits_max() / 5 //
// <= than 80% == 10 parts }

// fn move_to_exit(creep: &Creep) {
//     if let Some(closest_exit) = creep.pos().find_closest_by_path(find::EXIT,
// None) {         info!("{} pos: {}, closest_exit x:{}, y: {}", creep.name(),
// creep.pos(), closest_exit.x(), closest_exit.y());         let _ =
// creep.move_to(closest_exit);     }
// }
