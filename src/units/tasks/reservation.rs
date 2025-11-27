use log::*;
use screeps::{Creep, HasPosition, ObjectId, Position, StructureController, SharedCreepProperties, OwnedStructureProperties};
use crate::{
    units::{Task, TaskResult, roles::Role, tasks::find_closest_exit},
    movement::walker::Walker
};


pub fn claim(id: ObjectId<StructureController>, position: Position, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if !creep.pos().is_near_to(position) {
        let goal = Walker::Exploring(false).walk(position, 1, creep, role, enemies);
        TaskResult::StillWorking(Task::Claim(id, position), Some(goal))
    } else if let Some(controller) = id.resolve() {
        match creep.claim_controller(&controller) {
            Ok(()) => {
                TaskResult::ResolveRequest(Task::Claim(id, position), false)
            },
            Err(err) => {
                error!("creep: {}, claim controller error: {:?}", creep.name(), err);
                TaskResult::StillWorking(Task::Claim(id, position), None)
            }
        }
    } else {
        warn!("{} weird reservation case {}", creep.name(), creep.pos().room_name());
        TaskResult::Abort
    }
}

pub fn book(id: ObjectId<StructureController>, position: Position, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    if role.get_home().is_some_and(|home| home != creep.pos().room_name()) && !enemies.is_empty() {
        if let Some(closest_exit) = find_closest_exit(creep, None) {
            let goal = Walker::Exploring(false).walk(closest_exit, 0, creep, role, enemies);
            TaskResult::StillWorking(Task::Escape(closest_exit), Some(goal))
        } else {
            warn!("{} no exit found in room {}", creep.name(), creep.pos().room_name());
            TaskResult::Abort
        }
    } else if !creep.pos().is_near_to(position) {
        let goal = Walker::Exploring(false).walk(position, 1, creep, role, enemies);
        TaskResult::StillWorking(Task::Book(id, position), Some(goal))
    } else if let Some(controller) = id.resolve() {
        if !controller.my() && controller.level() > 0 {
            let _ = creep.attack_controller(&controller);
            TaskResult::StillWorking(Task::Book(id, position), None)
        } else {
            match creep.reserve_controller(&controller) {
                Ok(()) => {
                    if controller.reservation().is_some_and(|reservation| reservation.ticks_to_end() > 3500) {
                        TaskResult::ResolveRequest(Task::Book(id, position), false)
                    } else {
                        TaskResult::StillWorking(Task::Book(id, position), None)
                    }
                },
                Err(err) => {
                    error!("creep: {}, reserve controller error: {:?}", creep.name(), err);
                    TaskResult::StillWorking(Task::Book(id, position), None)
                }
            }
        }
    } else {
        warn!("{} weird reservation case {}", creep.name(), creep.pos().room_name());
        TaskResult::Abort
    }
}