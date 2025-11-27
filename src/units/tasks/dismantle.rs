use log::*;
use screeps::{Creep, HasPosition, StructureObject, ObjectId, Position, SharedCreepProperties, Structure};
use crate::{
    units::{Task, TaskResult, roles::Role},
    movement::walker::Walker
};

pub fn dismantle(id: ObjectId<Structure>, workplace: Position, creep: &Creep, role: &Role) -> TaskResult {
    if creep.pos().is_equal_to(workplace) {
        if let Some(structure) = id.resolve().map(|str: Structure| StructureObject::from(str)) {
            if let Some(dismantleable) = structure.as_dismantleable() {
                let _ = creep.dismantle(dismantleable);
                let goal = Walker::Immobile.walk(workplace, 0, creep, role, Vec::new());
                TaskResult::StillWorking(Task::Dismantle(id, workplace), Some(goal))
            } else {
                error!("{} structure: {} isn't dismantleable! resolve dismantle request!", creep.name(), id);
                TaskResult::ResolveRequest(Task::Dismantle(id, workplace), false)
            }
        } else {
            error!("{} not found structure: {}, resolve dismantle request!", creep.name(), id);
            TaskResult::ResolveRequest(Task::Dismantle(id, workplace), false)
        }
    } else {
        let another_task = Task::PullTo(creep.name(), workplace);
        let goal = Walker::Immobile.walk(workplace, 0, creep, role, Vec::new());
        TaskResult::AddNewRequest(Task::Dismantle(id, workplace), another_task, Some(goal))
    }
}