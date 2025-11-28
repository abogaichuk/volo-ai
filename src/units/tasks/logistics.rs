use log::*;
use std::cmp;
use wasm_bindgen::JsCast;
use screeps::{
    Creep, HasId, HasPosition, HasStore, ObjectId, Position, RawObjectId,
    Resource, ResourceType, SharedCreepProperties, StructureContainer,
    StructureController, action_error_codes::WithdrawErrorCode, game
};
use crate::{
    movement::walker::Walker, rooms::wrappers::Fillable, units::{Task, TaskResult, roles::Role}, utils::{
        commons::{find_walkable_positions_near_by, get_in_room_bank},
        constants::CLOSE_RANGE_ACTION}
};

pub fn take_resource(id: ObjectId<Resource>, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if let Some(resource) = id.resolve() {
        if creep.pos().is_near_to(resource.pos()) {
            match creep.pickup(&resource) {
                Ok(_) => {
                    let _ = creep.say("💰", false); //money bag
                    TaskResult::ResolveRequest(Task::TakeResource(id), false)
                }
                Err(err) => {
                    error!("creep: {} can't pickup resource: {:?}, error: {:?}", creep.name(), resource, err);
                    TaskResult::ResolveRequest(Task::TakeResource(id), false)
                }
            }
        } else {
            let goal = Walker::Exploring(false).walk(resource.pos(), CLOSE_RANGE_ACTION, creep, role, enemies);
            TaskResult::StillWorking(Task::TakeResource(id), Some(goal))
        }
    } else {
        error!("{} resource not found! {}", creep.name(), id);
        TaskResult::ResolveRequest(Task::TakeResource(id), false)
    }
}

pub fn take_from_structure(pos: Position, id: RawObjectId, resource: ResourceType, amount: Option<u32>, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    //todo handle case if creep is full!
    //todo if creep_mem.goal.len() > tick_to_live -> drop request
    if let Some(room_obj) = game::get_object_by_id_erased(&id) {
        let container = room_obj.unchecked_ref::<StructureContainer>();
        if amount.is_none_or(|amount| container.store().get_used_capacity(Some(resource)) >= amount) {
            if creep.pos().is_near_to(room_obj.pos()) {
                match creep.withdraw(container, resource, amount) {
                    Ok(_) => {
                        let _ = creep.say("💰", false); //money bag
                        debug!("{} money bag {} id {}", creep.name(), resource, id.to_string());
                        TaskResult::Completed
                    }
                    Err(err) => {
                        match err {
                            //additional attempt to withdraw at least some amount of resources
                            WithdrawErrorCode::NotEnoughResources => {
                                match creep.withdraw(container, resource, None) {
                                    Ok(_) => {
                                        info!("{} additional attempt to whithdow {} from pos {}", creep.name(), resource, pos);
                                        let _ = creep.say("💰", false); //money bag
                                        TaskResult::Completed
                                    }
                                    _ => {
                                        if game::time() % 10 == 0 {
                                            error!("creep: {} can't withdraw resource: {} from: {}, error: {:?}", creep.name(), resource, id, err);
                                        }
                                        TaskResult::Abort
                                    }
                                }
                            }
                            _ => {
                                error!("creep: {} can't withdraw resource: {} from: {}, error: {:?}", creep.name(), resource, id, err);
                                TaskResult::Abort
                            }
                        }
                    }
                }
            } else {
                let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
                TaskResult::StillWorking(Task::TakeFromStructure(pos, id, resource, amount), Some(goal))
            }
        } else {
            warn!("{} drop take_from_structure: {} not enough: {}, amount: {:?}", creep.name(), id, resource, amount);
            TaskResult::Abort
        }
    } else if creep.pos().room_name() != pos.room_name() {
        let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::TakeFromStructure(pos, id, resource, amount), Some(goal))
    } else {
        error!("{} container take from not found! {}", creep.name(), id);
        TaskResult::Abort
    }
}

pub fn deliver_to_structure(pos: Position, id: RawObjectId, resource: ResourceType, amount: Option<u32>, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if let Some(room_obj) = game::get_object_by_id_erased(&id) {
        let container = room_obj.unchecked_ref::<StructureContainer>();
        if container.store().get_free_capacity(Some(resource)) == 0 {
            TaskResult::Abort
        } else if creep.pos().is_near_to(room_obj.pos()) {
            match creep.transfer(container, resource, amount) {
                Ok(_) => {
                    let _ = creep.say("👌", false); //OK emoji!
                    TaskResult::Completed
                }
                Err(err) => {
                    error!("{} can't transfer resource: {} to: {}, error: {:?}", creep.name(), resource, id, err);
                    TaskResult::Abort
                }
            }
        } else {
            let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
            TaskResult::StillWorking(Task::DeliverToStructure(pos, id, resource, amount), Some(goal))
        }
    } else if creep.pos().room_name() != pos.room_name() {
        let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::DeliverToStructure(pos, id, resource, amount), Some(goal))
    } else {
        error!("{} container deliver to not found! {}", creep.name(), id);
        TaskResult::Abort
    }
}

// pub fn deliver_to_structure(pos: Position, id: RawObjectId, resource: ResourceType, amount: Option<u32>, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
//     if let Some(room_obj) = game::get_object_by_id_erased(&id) {
//         if creep.pos().is_near_to(room_obj.pos()) {
//             let container = room_obj.unchecked_ref::<StructureContainer>();
//             match creep.transfer(container, resource, amount) {
//                 Ok(_) => {
//                     let _ = creep.say("👌", false); //OK emoji!
//                     TaskResult::Completed
//                 }
//                 Err(err) => {
//                     error!("{} can't transfer resource: {} to: {}, error: {:?}", creep.name(), resource, id, err);
//                     TaskResult::Abort
//                 }
//             }
//         } else {
//             let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
//             TaskResult::StillWorking(Task::DeliverToStructure(pos, id, resource, amount), Some(goal))
//         }
//     } else if creep.pos().room_name() != pos.room_name() {
//         let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
//         TaskResult::StillWorking(Task::DeliverToStructure(pos, id, resource, amount), Some(goal))
//     } else {
//         error!("{} container deliver to not found! {}", creep.name(), id);
//         TaskResult::Abort
//     }
// }

pub fn fill_structure(structure: Box<dyn Fillable>, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if creep.pos().is_near_to(structure.position()) {
        let _ = creep.say("🚰", false); //valve
        let _ = creep.transfer(structure.as_transferable(), ResourceType::Energy, None);
        TaskResult::Completed
    } else {

        let _ = creep.say("🚶🏿", false); //walk dark
        let goal = Walker::Exploring(false).walk(structure.position(), CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::DeliverToStructure(structure.position(), structure.id(), ResourceType::Energy, None), Some(goal))
        // TaskResult::MoveTo(goal)
    }
    // let cpu_start = game::cpu::get_used();
    // let home_room = game::rooms().get(room_name).expect("expect home room!");
    
    // if creep.store().get_used_capacity(Some(ResourceType::Energy)) < extension_capacity(&home_room) {
    //     TaskResult::Abort
    // } else if let Some(closest_str) = find_closest_empty_structure(&home_room, creep) {
    //     if creep.pos().is_near_to(closest_str.pos()) {
    //         if let Some(transferable) = closest_str.as_transferable() {
                // let _ = creep.say("🚰", false); //valve
                // let _ = creep.transfer(transferable, ResourceType::Energy, None);
    //             let cpu_used = game::cpu::get_used() - cpu_start;
    //             if cpu_used > 5. {
    //                 info!("{}, fill_structures transfer - pos: {}, cpu {} exceed!", creep.name(), creep.pos(), cpu_used);
    //             }
    //             TaskResult::StillWorking(Task::FillStructures(room_name), None)
    //         } else {
    //             error!("{} structure: {} is not transferable", creep.name(), closest_str.as_structure().raw_id());
    //             TaskResult::Abort
    //         }
    //     } else {
            // let _ = creep.say("🚶🏿", false); //walk dark
            // let goal = Walker::Exploring(false).walk(closest_str.pos(), CLOSE_RANGE_ACTION, creep, role, enemies);
    //         let cpu_used = game::cpu::get_used() - cpu_start;
    //         if cpu_used > 5. {
    //             info!("{}, fill_structures walk - pos: {}, to: {}, cpu {} exceed!", creep.name(), creep.pos(), goal.pos, cpu_used);
    //         }
    //         TaskResult::StillWorking(Task::FillStructures(room_name), Some(goal))
    //     }
    // } else {
    //     TaskResult::Abort
    // }
}

// pub fn fill_structures(room_name: RoomName, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
//     let cpu_start = game::cpu::get_used();
//     let home_room = game::rooms().get(room_name).expect("expect home room!");
    
//     if creep.store().get_used_capacity(Some(ResourceType::Energy)) < extension_capacity(&home_room) {
//         TaskResult::Abort
//     } else if let Some(closest_str) = find_closest_empty_structure(&home_room, creep) {
//         if creep.pos().is_near_to(closest_str.pos()) {
//             if let Some(transferable) = closest_str.as_transferable() {
//                 let _ = creep.say("🚰", false); //valve
//                 let _ = creep.transfer(transferable, ResourceType::Energy, None);
//                 let cpu_used = game::cpu::get_used() - cpu_start;
//                 if cpu_used > 5. {
//                     info!("{}, fill_structures transfer - pos: {}, cpu {} exceed!", creep.name(), creep.pos(), cpu_used);
//                 }
//                 TaskResult::StillWorking(Task::FillStructures(room_name), None)
//             } else {
//                 error!("{} structure: {} is not transferable", creep.name(), closest_str.as_structure().raw_id());
//                 TaskResult::Abort
//             }
//         } else {
//             let _ = creep.say("🚶🏿", false); //walk dark
//             let goal = Walker::Exploring(false).walk(closest_str.pos(), CLOSE_RANGE_ACTION, creep, role, enemies);
//             let cpu_used = game::cpu::get_used() - cpu_start;
//             if cpu_used > 5. {
//                 info!("{}, fill_structures walk - pos: {}, to: {}, cpu {} exceed!", creep.name(), creep.pos(), goal.pos, cpu_used);
//             }
//             TaskResult::StillWorking(Task::FillStructures(room_name), Some(goal))
//         }
//     } else {
//         TaskResult::Abort
//     }
// }

pub fn withdraw(pos: Position, id: RawObjectId, mut resources: Vec<(ResourceType, Option<u32>)>, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if let Some((resource, amount)) = resources.first() {
        match take_from_structure(pos, id, *resource, *amount, creep, role, enemies) {
            TaskResult::StillWorking(_, movement_goal) => {
                TaskResult::StillWorking(Task::Withdraw(pos, id, resources), movement_goal)
            }
            TaskResult::Completed => {
                resources.remove(0);
                if resources.is_empty() {
                    TaskResult::ResolveRequest(Task::Withdraw(pos, id, resources), false)
                } else {
                    TaskResult::UpdateRequest(Task::Withdraw(pos, id, resources))
                }

            }
            _ => {
                error!("{} withdraw got abort from take from: {:?} task, resource: {}", creep.name(), id, resource);
                TaskResult::ResolveRequest(Task::Withdraw(pos, id, resources), false)
            }
        }
    } else {
        TaskResult::ResolveRequest(Task::Withdraw(pos, id, resources), false)
    }
}

pub fn long_range_withdraw(pos: Position, id: RawObjectId, resource: ResourceType, amount: u32, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    let take_amount = cmp::min(creep.store().get_free_capacity(None) as u32, amount);
    debug!("{} lrw take amount: {}, amount: {}", creep.name(), take_amount, amount);
    match take_from_structure(pos, id, resource, Some(take_amount), creep, role, enemies) {
        TaskResult::StillWorking(_, movement_goal) => {
            TaskResult::StillWorking(Task::LongRangeWithdraw(pos, id, resource, amount), movement_goal)
        }
        TaskResult::Completed => {
            info!("{} Completed lrw take amount: {}, amount: {}", creep.name(), take_amount, amount);
            if take_amount < amount {
                TaskResult::UpdateRequest(Task::LongRangeWithdraw(pos, id, resource, amount - take_amount))
            } else {
                TaskResult::ResolveRequest(Task::LongRangeWithdraw(pos, id, resource, amount), false)
            }
        }
        _ => {
            error!("{} withdraw got abort from take from: {:?} task, resource: {}", creep.name(), id, resource);
            TaskResult::ResolveRequest(Task::LongRangeWithdraw(pos, id, resource, amount), false)
        }
    }
}

pub fn generate_safe_mode(pos: Position, id: ObjectId<StructureController>, storage_id: RawObjectId, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if creep.store().get_used_capacity(Some(ResourceType::Ghodium)) >= 1000 {
        if let Some(controller) = id.resolve() {
            if creep.pos().is_near_to(controller.pos()) {
                match creep.generate_safe_mode(&controller) {
                    Ok(_) => {
                        let _ = creep.say("👌", false); //OK emoji!
                        TaskResult::ResolveRequest(Task::GenerateSafeMode(pos, id, storage_id), true)
                    }
                    Err(err) => {
                        error!("{}: {} Generate safe mode failed! error: {:?}", creep.name(), id, err);
                        TaskResult::Abort
                    }
                }
            } else {
                let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
                TaskResult::StillWorking(Task::GenerateSafeMode(pos, id, storage_id), Some(goal))
            }
        } else if creep.pos().room_name() != pos.room_name() {
            let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
            TaskResult::StillWorking(Task::GenerateSafeMode(pos, id, storage_id), Some(goal))
        } else {
            error!("{}, generate safe mode failed! Controller not found! {}", creep.name(), id);
            TaskResult::Abort
        }
    } else if let Some(room_obj) = game::get_object_by_id_erased(&storage_id) {
        let container = room_obj.unchecked_ref::<StructureContainer>();
        let goal = Walker::Exploring(false).walk(container.pos(), CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::TakeFromStructure(container.pos(), storage_id, ResourceType::Ghodium, Some(1000)), Some(goal))
    } else {
        TaskResult::Abort
    }
}

pub fn carry(from: RawObjectId, to: RawObjectId, resource: ResourceType, amount: u32, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    debug!("{} start carry {} from {} to {}", creep.name(), resource, from.to_string(), to.to_string());
    if has_enough_or_full(creep, resource, amount) {
        //deliver to structure
        if let Some(room_obj) = game::get_object_by_id_erased(&to) {
            let current_amount = cmp::min(amount, creep.store().get_used_capacity(Some(resource)));
            match deliver_to_structure(room_obj.pos(), to, resource, Some(current_amount), creep, role, enemies) {
                TaskResult::Completed => {
                    if current_amount >= amount {
                        TaskResult::ResolveRequest(Task::Carry(from, to, resource, 0, None), false)
                    } else {
                        TaskResult::UpdateRequest(Task::Carry(from, to, resource, amount - current_amount, None))
                    }
                }
                TaskResult::StillWorking(_, movement_goal) => {
                    TaskResult::StillWorking(Task::Carry(from, to, resource, amount, None), movement_goal)
                }
                _ => {
                    error!("{} carry got abort from deliver to: {:?} task", creep.name(), to);
                    TaskResult::ResolveRequest(Task::Carry(from, to, resource, amount, None), false)
                }
            }
        } else {
            error!("there is no structure carry to: {:?}", to);
            TaskResult::ResolveRequest(Task::Carry(from, to, resource, amount, None), false)
        }
    } else if let Some(useless_res) = get_resource(creep, &resource) {
        //unload the useless_res
        let home_room = role.get_home()
            .and_then(|home| game::rooms().get(home))
            .expect("expect role has a home!");
        
        if let Some(storage) = get_in_room_bank(&home_room) {
            match deliver_to_structure(storage.pos(), storage.as_structure().raw_id(), useless_res, None, creep, role, enemies) {
                TaskResult::Abort => {
                    TaskResult::ResolveRequest(Task::Carry(from, to, resource, amount, None), false)
                }
                another => another
            }
        } else {
            error!("{} there is no available bank in: {:?}", creep.name(), role.get_home());
            TaskResult::ResolveRequest(Task::Carry(from, to, resource, amount, None), false)
        }
    } else if let Some(room_obj) = game::get_object_by_id_erased(&from) {
        //creep has or has not some amount of the resource here, withdraw additional resource
        let additional_amount = if amount > creep.store().get_capacity(None) {
            None
        } else {
            Some(amount - creep.store().get_used_capacity(Some(resource)))
        };
        match take_from_structure(room_obj.pos(), from, resource, additional_amount, creep, role, enemies) {
            TaskResult::Abort => {
                error!("{} carry got abort from take_from_structure {}, res: {}, amount: {}", creep.name(), from, resource, amount);
                TaskResult::ResolveRequest(Task::Carry(from, to, resource, amount, None), false)
            }
            another => another
        }
    } else {
        error!("there is no structure carry from: {:?}", from);
        TaskResult::ResolveRequest(Task::Carry(from, to, resource, amount, None), false)
    }
}

pub fn pull_to(creep_name: String, destination: Position, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if let Some(cargo) = game::creeps().get(creep_name.clone()) {
        if !cargo.pos().is_equal_to(destination) {
            if creep.pos().is_near_to(cargo.pos()) {
                if creep.pos().is_equal_to(destination) {
                    let _ = creep.pull(&cargo);
                    let _ = cargo.move_pulled_by(creep);

                    let goal = Walker::Exploring(false).walk(cargo.pos(), 0, creep, role, enemies);
                    TaskResult::StillWorking(Task::PullTo(creep_name, destination), Some(goal))
                } else if cargo.pos().is_room_edge() && creep.pos().is_room_edge() {
                    let _ = creep.pull(&cargo);
                    let _ = cargo.move_pulled_by(creep);

                    let goal = Walker::Exploring(false).walk(destination, 0, creep, role, enemies);
                    TaskResult::StillWorking(Task::PullTo(creep_name, destination), Some(goal))
                } else if creep.pos().is_room_edge() {
                    if let Some(beside) = get_edge_position(creep.pos()) {
                        let _ = creep.pull(&cargo);
                        let _ = cargo.move_pulled_by(creep);

                        let goal = Walker::Exploring(false).walk(beside.pos(), 0, creep, role, enemies);
                        TaskResult::StillWorking(Task::PullTo(creep_name, destination), Some(goal))
                    } else {
                        warn!("creep: {} there is no available position, waiting..", creep.name());
                        TaskResult::StillWorking(Task::PullTo(creep_name, destination), None)
                    }
                }
                else {
                    let _ = creep.pull(&cargo);
                    let _ = cargo.move_pulled_by(creep);

                    let goal = Walker::Exploring(false).walk(destination, 0, creep, role, enemies);
                    TaskResult::StillWorking(Task::PullTo(creep_name, destination), Some(goal))
                }
            } else if !cargo.pos().is_room_edge() {
                let goal = Walker::Exploring(false).walk(cargo.pos(), 1, creep, role, enemies);
                TaskResult::StillWorking(Task::PullTo(creep_name, destination), Some(goal))
            } else {
                //just wait because there is no cargo near(means puller is in other room)
                let _ = creep.say("🚬", false);
                TaskResult::StillWorking(Task::PullTo(creep_name, destination), None)
            }
        } else {
            info!("creep: {} cargo {} reached the destination {:?}", creep.name(), creep_name, destination);
            // let _ = creep.suicide();
            TaskResult::ResolveRequest(Task::PullTo(creep_name, destination), false)
        }
    } else {
        //todo resolve??
        TaskResult::StillWorking(Task::PullTo(creep_name, destination), None)
    }
}

fn has_enough_or_full(creep: &Creep, resource: ResourceType, amount: u32) -> bool {
    creep.store().get_used_capacity(Some(resource)) >= amount
        || creep.store().get_used_capacity(Some(resource)) == creep.store().get_capacity(None)
}

fn get_resource(store: &dyn HasStore, exclude: &ResourceType) -> Option<ResourceType> {
    store.store().store_types().into_iter().find(|resource| resource != exclude)
}


fn get_edge_position(position: Position) -> Option<Position> {
    find_walkable_positions_near_by(position, false)
        .into_iter()
        .find(|pos| pos.x().u8() == 0 || pos.x().u8() == 49 || pos.y().u8() == 0 || pos.y().u8() == 49)
}