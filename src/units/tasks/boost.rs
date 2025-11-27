use log::*;
use screeps::{Creep, SharedCreepProperties, HasPosition, HasId, ObjectId, StructureLab};
use crate::{
    units::{Task, TaskResult, roles::{Role, Kind}},
    movement::{walker::Walker, MovementProfile},
    utils::{constants::CLOSE_RANGE_ACTION, commons::find_walkable_positions_near_by}};


pub fn boost(id: ObjectId<StructureLab>, parts_number: Option<u32>, creep: &Creep, role: &Role) -> TaskResult {
    if let Some(lab) = id.resolve() {
        if creep.pos().is_near_to(lab.pos()) {
            let boost_result = lab.boost_creep(creep, parts_number);
            info!("{} {} boosted: {:?}", creep.name(), id, boost_result);
            TaskResult::Completed
        } else if let MovementProfile::Cargo = role.get_movement_profile(creep) {
            if let Some(pos) = find_walkable_positions_near_by(lab.pos(), true).first() {
                let another_task = Task::PullTo(creep.name(), *pos);
                let goal = Walker::Immobile.walk(*pos, 0, creep, role, Vec::new());
                TaskResult::AddNewRequest(Task::Boost(lab.id(), parts_number), another_task, Some(goal))
            } else {
                let goal = Walker::Immobile.walk(lab.pos(), 1, creep, role, Vec::new());
                TaskResult::StillWorking(Task::Boost(lab.id(), parts_number), Some(goal))
            }
        } else {
            let goal = Walker::Exploring(false).walk(lab.pos(), CLOSE_RANGE_ACTION, creep, role, Vec::new());
            TaskResult::StillWorking(Task::Boost(lab.id(), parts_number), Some(goal))
        }
    } else {
        error!("{} {:?} invalid lab {}", creep.name(), creep.room(), id);
        TaskResult::Abort
    }
}