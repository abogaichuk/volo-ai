// use log::*;
// use screeps::{Resource, Tombstone};
// use screeps::{find, game, Creep, Direction, HasPosition, MoveToOptions, ObjectId, Part, Position, ResourceType,
//     Room, RoomName, RoomPosition, ScoreCollector, ScoreContainer, SharedCreepProperties,
//     Structure, StructureContainer, StructureObject, look::TOMBSTONES, look::RESOURCES, look::CREEPS,
//     pathfinder::SingleRoomCostResult, FindPathOptions, Path::Vectorized,
//     pathfinder::SearchOptions, pathfinder};
// use crate::memory::{self, ClaimedMemory, CreepMemory, Executable, PowerbankRequest, RequestStatus, Resolvable, RoomRequest, RoomState, ScoreRequest, Target, Task};
// use crate::utils::commons::{self, get_random};
// use screeps::look::LookResult;
// use crate::creep_handler::unload_all;
// use std::collections::HashMap;
// use std::str::FromStr;
// use std::cmp;
// use crate::utils::constants::INVANDER;
// use super::{flee, is_dying, get_active_job, get_home_room_name, DISTANCE_TO_HOSTILE, get_home_room_name2};

// const zone2: &'static [&'static str] = &[
// ];
// pub fn run(creep: &Creep, memory: &mut CreepMemory, rooms_state: &mut HashMap<RoomName, RoomState>) {
//     let home_room_name = get_home_room_name2(memory.parent_room.as_ref());
//     let home_room_state = commons::get_claimed_room_state(home_room_name, rooms_state);

//     let room = creep.room().expect("couldn't resolve a room");

//     let mut enemies = commons::find_hostiles(&room, vec![Part::Attack, Part::RangedAttack]).collect::<Vec<Creep>>();
//     let invander = get_closest(creep, &mut enemies);

//     if home_room_name.to_string() == ""{

//         if creep.store().get_used_capacity(Some(ResourceType::Score)) > 0 {
//             let room = game::rooms().get(home_room_name).expect("couldn't resolve a home room");
//             if let Some(container) = commons::get_place_to_store(&room) {
//                 if creep.pos().is_near_to(container.pos()) {
//                     if let Some(transferable) = container.as_transferable() {
//                         let _ = creep.transfer(transferable, ResourceType::Score, None);
//                     }
//                 } else {
//                     let _ = creep.move_to_with_options(container, Some(MoveToOptions::new()
//                         .plain_cost(0)
//                         .swamp_cost(3)
//                         .reuse_path(get_random(4, 6) as u32)));
//                     // let _ = creep.move_to_with_options(container, Some(MoveToOptions::new().reuse_path(4)));
//                 }
//             }
//         } else {
//             // find_score(creep: &Creep, room: &Room, home_room_name: RoomName, home_room_state: &mut ClaimedMemory, invander: Option<&Creep>)
//             find_score(creep, &room, home_room_name, home_room_state, invander, &memory);
//         }
//     } else {
//         if let Some(target) = get_score_collector(home_room_name, memory) {
//             let collector_position: Position = RoomPosition::new(target.x, target.y, target.room_name).into();

//             if creep.store().get_used_capacity(Some(ResourceType::Score)) > 0 {
//                 if collector_position.room_name().to_string() == "" && creep.pos().get_range_to(collector_position) <= 7 {
//                 // if collector_position.room_name().to_string() == "" && creep.pos().get_range_to(collector_position) <= 5 {
//                 // if creep.pos().get_range_to(collector_position) <= 7 {
//                 // if target_position.room_name() == room.name() {
//                     if creep.pos().is_near_to(collector_position) {
//                         let score_collector:Option::<ScoreCollector> = ObjectId::from_str(&target.id).ok()
//                             .map_or_else(|| None, |o_id| o_id.resolve());
        
//                         if let Some(score_collector) = score_collector {
//                             let _ = creep.transfer(&score_collector, ResourceType::Score, None);
//                         } else {
//                             warn!("creep: {}, incorrect score collector: {:?}", creep.name(), target);
//                         }
//                     } else {
//                         move_near_collector(creep, collector_position);
//                     }
//                 } else if creep.pos().get_range_to(collector_position) <= 5 {
//                     // if creep.pos().get_range_to(collector_position) <= 7 {
//                     // if target_position.room_name() == room.name() {
//                         if creep.pos().is_near_to(collector_position) {
//                             let score_collector:Option::<ScoreCollector> = ObjectId::from_str(&target.id).ok()
//                                 .map_or_else(|| None, |o_id| o_id.resolve());
            
//                             if let Some(score_collector) = score_collector {
//                                 let _ = creep.transfer(&score_collector, ResourceType::Score, None);
//                             } else {
//                                 warn!("creep: {}, incorrect score collector: {:?}", creep.name(), target);
//                             }
//                         } else {
//                             move_near_collector(creep, collector_position);
//                         }
//                     } else {
//                     let _ = creep.move_to_with_options(collector_position, Some(MoveToOptions::new()
//                         .plain_cost(0)
//                         .swamp_cost(3)
//                         .reuse_path(get_random(5,8) as u32))
//                     );
//                 }
//             } else {
//                 find_score(creep, &room, home_room_name, home_room_state, invander, &memory);
//             }
//         } else {
//             warn!("creep: {} score collector not found", creep.name());
//         }
//     }
// }

// fn move_near_collector(creep: &Creep, pos: Position) {
//     let path_options: std::option::Option<FindPathOptions<fn(RoomName, screeps::CostMatrix) -> 
//         SingleRoomCostResult, SingleRoomCostResult>> = Some(FindPathOptions::default()
//             .ignore_creeps(true).range(1));
//     let path = RoomPosition::from(creep.pos())
//         .find_path_to_xy(pos.x(), pos.y(), path_options);
//     debug!("full creep: {} is going to collector path: {:?}", creep.name(), path);

//     let _ = match path {
//         Vectorized(steps) => {
//             steps.first().and_then(|step| {
//                 debug!("full creep: {} step: {:?}", creep.name(), step);
//                 let next_pos = creep.pos() + step.direction;
//                 if let Some(obstacle) = find_obstacle(creep, &next_pos) {
//                     debug!("full creep: {} obstacle: {:?}", creep.name(), obstacle.name());
//                     swap_move(&obstacle, -step.direction);
//                 }
//                 creep.move_direction(step.direction).ok()
//             })
//         },
//         _ => { None }
//     };
// }

// fn find_obstacle(me: &Creep, position: &Position) -> Option<Creep> {
//     position.look_for(CREEPS).ok()
//         .and_then(|creeps| creeps.into_iter()
//             .find(|creep| creep.my()
//                 && (has_part(creep, Part::RangedAttack)
//                     || (me.store().get_used_capacity(None) > 0 && creep.store().get_used_capacity(None) == 0))))
// }

// fn find_score(creep: &Creep, room: &Room, home_room_name: RoomName, home_room_state: &mut ClaimedMemory, invander: Option<&Creep>, memory: &CreepMemory) {
//     let tomb = room.find(find::TOMBSTONES, None)
//         .into_iter()
//         .find(|tomb| tomb.store().get_used_capacity(Some(ResourceType::Score)) > 400);
//     let resource = room.find(find::DROPPED_RESOURCES, None)
//         .into_iter()
//         .find(|resource| resource.resource_type() == ResourceType::Score && resource.amount() >= 400);

//     if let Some(tomb) = tomb {
//         debug!("creep: {} tomb found pos: {}", creep.name(), tomb.pos());
//         if creep.pos().is_near_to(tomb.pos()) {
//             let _ = creep.withdraw(&tomb, ResourceType::Score, None);
//         } else {
//             let _ = creep.move_to(tomb);
//         }
//     } else if let Some(resource) = resource {
//         debug!("creep: {} resource found pos: {}", creep.name(), resource.pos());
//         if creep.pos().is_near_to(resource.pos()) {
//             let _ = creep.pickup(&resource);
//         } else {
//             let _ = creep.move_to(resource);
//         }
//     } 
//     // else if let Some(score_container) = room.find(find::SCORE_CONTAINERS, None).first() {
//     //     debug!("creep: {} score_container found pos: {}", creep.name(), score_container.pos());
//     //     if creep.pos().is_near_to(score_container.pos()) {
//     //         let _ = creep.withdraw(score_container, ResourceType::Score, None);
//     //     } else {
//     //         let _ = creep.move_to(score_container);
//     //     }
//     // } 
//     else if let Some(score_request) = get_active_request(&mut home_room_state.requests, memory) {
//         execute_request(creep, score_request, invander);
//     } else {
//         let _ = creep.say("🚬", false);
//         // let _ = creep.say("smoke..", false);

//         if home_room_name.to_string() == "" {
//             if creep.ticks_to_live().is_some_and(|ticks| ticks < 200) {
//                 let _ = creep.suicide();
//                 return;
//             }
//             let home_room = game::rooms().get(home_room_name).expect("expect home room");
//             if let Some(container) = commons::find_container_with(&home_room, ResourceType::Score, None) {
//                 if creep.pos().is_near_to(container.pos()) {
//                     if let Some(withdrawable) = container.as_withdrawable() {
//                         let _ = creep.withdraw(withdrawable, ResourceType::Score, None);
//                     }
//                 } else {
//                     //commons::get_random(1, 3) as u32
//                     let _ = creep.move_to_with_options(container.pos(), Some(MoveToOptions::new().plain_cost(0).swamp_cost(2).reuse_path(get_random(4, 6) as u32)));
//                 }
//             } else {
//                 // let controller = game::rooms().get(home_room_name).and_then(|room| room.controller()).expect("couldn't resolve a home room");
//                 let _ = creep.move_to_with_options(
//                     get_wait_position(home_room_name, memory),
//                     Some(MoveToOptions::new()
//                         .plain_cost(0)
//                         .swamp_cost(2)
//                         .range(3)
//                         .reuse_path(get_random(4, 6) as u32)));
//             }
//         } else {
//             // let controller = game::rooms().get(home_room_name).and_then(|room| room.controller()).expect("couldn't resolve a home room");
//             let _ = creep.move_to_with_options(
//                 get_wait_position(home_room_name, memory),
//                 Some(MoveToOptions::new()
//                     .plain_cost(0)
//                     .swamp_cost(2)
//                     .range(3)
//                     .reuse_path(get_random(4, 6) as u32)));
//         }
//     }
// }

// fn has_part(enemy: &Creep, part: Part) -> bool {
//     enemy.body().iter()
//         .map(|bodypart| {
//             if bodypart.hits() > 0 {
//                 Some(bodypart.part())
//             } else {
//                 None
//             }
//         })
//         .any(|bp| bp.is_some_and(|bp| bp == part))
// }

// fn swap_move(creep: &Creep, direction: Direction) {
//     let _ = creep.move_direction(direction);
//     let _ = creep.say(format!("{}", direction).as_str(), true);
// }

// pub fn get_wait_position(home_room_name: RoomName, memory: &CreepMemory) -> Position {
//     if let Some(target_pos) = memory.target.as_ref().map(|target| RoomPosition::new(target.x, target.y, target.room_name).into()){
//         target_pos
//     } 
//     else {
//         game::rooms().get(home_room_name).and_then(|room| room.controller()).expect("couldn't resolve a home room").pos()
//     }
// }

// pub fn get_closest<'a>(to: &dyn HasPosition, enemies: &'a mut Vec<Creep>) -> Option<&'a Creep> {
//     enemies.sort_by_key(|enemy| enemy.pos().get_range_to(to.pos()));
//     enemies.first()
// }

// fn execute_request(creep: &Creep, request: &mut ScoreRequest, invander: Option<&Creep>) {
//     if creep.ticks_to_live().is_some_and(|ticks| ticks < 2) {
//         request.doer = None;
//         let _ = creep.suicide();
//         return;
//     }
//     let target_position = RoomPosition::new(request.target.x, request.target.y, request.target.room_name).into();
//     // if creep.room().expect("expect creep in a room").name() == request.target.room_name

//     //todo check for hostiles nearby
//     if creep.pos().is_near_to(target_position) {
//         let target:Option<ScoreContainer> = ObjectId::from_str(request.target.id.as_str()).ok()
//             .map_or_else(||None, |o_id| o_id.resolve());

//         if let Some(score_container) = target {
//             let result = creep.withdraw(&score_container, ResourceType::Score, None);

//             match result {
//                 Ok(_) => { 
//                     // info!("creep: {} pos: {:?} score amount: {:?}", creep.name(), creep.pos(), request.amount);
//                     let amount = score_container.store().get_used_capacity(None);
//                     // info!("creep: {} pos: {:?} score amount: {:?}", creep.name(), creep.pos(), amount);
//                     if amount > creep.store().get_free_capacity(None) as u32 {
//                         request.amount = amount - creep.store().get_free_capacity(None) as u32;
//                     } else {
//                         request.amount = 0;
//                         debug!("creep: {} close score request: {:?}", creep.name(), request);
//                         request.status = RequestStatus::Resolved;
//                     }
//                 },
//                 Err(error) => {
//                     warn!("creep: {} couldn't withdraw: {:?}", creep.name(), error);
//                     request.status = RequestStatus::Resolved;
//                 }
//             }
//         } else {
//             request.status = RequestStatus::Resolved;
//         }
//     } else {
//         if let Some(enemy) = invander {
//             if enemy.owner().username() != INVANDER {
//                 let range = creep.pos().get_range_to(enemy.pos());
//                 if range < 5 {
//                     flee(creep, enemy.pos(), 10);
//                 }
//             }
//         } 
//         // let _ = creep.move_to_with_options(target_position, Some(MoveToOptions::new().reuse_path(1)));
//         let _ = creep.move_to_with_options(target_position, Some(MoveToOptions::new().plain_cost(0).swamp_cost(1).reuse_path(get_random(4, 6) as u32)));
//     }
// }

// fn get_score_collector(room_name: RoomName, memory: &CreepMemory) -> Option<Target> {
//     // if memory.target.is_some() {
//     //     return memory.target.clone();
//     // }
// }

// fn get_active_request<'a>(requests: &'a mut Vec<RoomRequest>, memory: &CreepMemory) -> Option<&'a mut ScoreRequest> {
//     requests.iter_mut()
//         .flat_map(|request| {
//             match request {
//                 RoomRequest::SCORE(sc) => {
//                     let zonable = zone2.contains(&sc.target.room_name.to_string().as_str());

//                     if let Some(target_room) = memory.target.as_ref().map(|target| target.room_name) {
//                         if target_room.to_string() == "" && zonable {
//                             Some(sc)
//                         } else {
//                             None
//                         }
//                     } else {
//                         // Some(sc)
//                         if !zonable {
//                             Some(sc)
//                         } else {
//                             None
//                         }
//                     }
//                 },
//                 _ => None
//             }
//         })
//         .find(|score_request|
//             score_request.status == RequestStatus::InProgress || score_request.status == RequestStatus::Created
//         )
// }

// // fn get_active_request<'a>(requests: &'a mut Vec<RoomRequest>, memory: &CreepMemory) -> Option<&'a mut ScoreRequest> {
// //     requests.iter_mut()
// //         .flat_map(|request| {
// //             match request {
// //                 RoomRequest::SCORE(sc) => Some(sc),
// //                 _ => None
// //             }
// //         })
// //         .find(|score_request|
// //             score_request.status == RequestStatus::InProgress || score_request.status == RequestStatus::Created
// //         )
// // }