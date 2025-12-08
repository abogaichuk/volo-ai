use log::*;
use screeps::{Creep, HasId, HasPosition, ObjectId, SharedCreepProperties, StructureLab};

use crate::movement::MovementProfile;
use crate::movement::walker::Walker;
use crate::units::roles::{Kind, Role};
use crate::units::{Task, TaskResult};
use crate::utils::commons::find_walkable_positions_near_by;
use crate::utils::constants::CLOSE_RANGE_ACTION;

pub fn boost(
    id: ObjectId<StructureLab>,
    parts_number: Option<u32>,
    creep: &Creep,
    role: &Role,
) -> TaskResult {
    if let Some(lab) = id.resolve() {
        if creep.pos().is_near_to(lab.pos()) {
            let boost_result = lab.boost_creep(creep, parts_number);
            info!("{} {} boosted: {:?}", creep.name(), id, boost_result);
            TaskResult::Completed
        } else if let MovementProfile::Cargo = role.get_movement_profile(creep) {
            if let Some(pos) = find_walkable_positions_near_by(lab.pos(), true).first() {
                let another_task = Task::PullTo(creep.name(), *pos);
                let goal = Walker::Immobile.walk(*pos, 0, creep, role, Vec::new());
                TaskResult::AddNewRequest(
                    Task::Boost(lab.id(), parts_number),
                    another_task,
                    Some(goal),
                )
            } else {
                let goal = Walker::Immobile.walk(lab.pos(), 1, creep, role, Vec::new());
                TaskResult::StillWorking(Task::Boost(lab.id(), parts_number), Some(goal))
            }
        } else {
            let goal = Walker::Exploring(false).walk(
                lab.pos(),
                CLOSE_RANGE_ACTION,
                creep,
                role,
                Vec::new(),
            );
            TaskResult::StillWorking(Task::Boost(lab.id(), parts_number), Some(goal))
        }
    } else {
        error!("{} {:?} invalid lab {}", creep.name(), creep.room(), id);
        TaskResult::Abort
    }
}
