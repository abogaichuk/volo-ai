use std::collections::HashSet;

use log::{error, warn};
use screeps::{
    Creep, HasPosition, ObjectId, Position, SharedCreepProperties, Structure, StructureObject, game,
};

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::{Task, TaskResult};
use crate::commons::find_closest_exit;

pub fn dismantle(
    id: ObjectId<Structure>,
    workplace: Position,
    creep: &Creep,
    role: &Role,
) -> TaskResult {
    if creep.pos().is_equal_to(workplace) {
        if let Some(structure) = id.resolve().map(|str: Structure| StructureObject::from(str)) {
            if let Some(dismantleable) = structure.as_dismantleable() {
                let _ = creep.dismantle(dismantleable);
                let goal = Walker::Immobile.walk(workplace, 0, creep, role, Vec::new());
                TaskResult::StillWorking(Task::Dismantle(id, workplace), Some(goal))
            } else {
                error!(
                    "{} structure: {} isn't dismantleable! resolve dismantle request!",
                    creep.name(),
                    id
                );
                TaskResult::ResolveRequest(Task::Dismantle(id, workplace))
            }
        } else {
            error!("{} not found structure: {}, resolve dismantle request!", creep.name(), id);
            TaskResult::ResolveRequest(Task::Dismantle(id, workplace))
        }
    } else {
        let another_task = Task::PullTo(creep.name(), workplace);
        let goal = Walker::Immobile.walk(workplace, 0, creep, role, Vec::new());
        TaskResult::AddNewRequest(Task::Dismantle(id, workplace), another_task, Some(goal))
    }
}

pub fn combat_dismantle(
    id: ObjectId<Structure>,
    workplace: Position,
    members: HashSet<String>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult
{
    if let Some(healer) = members
        .iter()
        .find(|name| **name != creep.name())
        .and_then(|member| game::creeps().get(member.clone()))
        && (creep.pos().is_near_to(healer.pos()) || creep.pos().is_room_edge())
    {
        // healer is near
        let is_injured = [creep, &healer].into_iter()
            .any(|c| ((c.hits() * 10) as f32) < c.hits_max() as f32 * 8.5);

        if is_injured {
            if creep.pos().room_name() != workplace.room_name() &&
                let Some(to_home_exit) = find_closest_exit(creep, role.get_home().copied())
            {
                let goal = Walker::Exploring(false).walk(to_home_exit, 0, creep, role, enemies);
                TaskResult::StillWorking(Task::CombatDismantle(id, workplace, members), Some(goal))
            } else if let Some(closest_exit) = find_closest_exit(creep, None) {
                let goal = Walker::Exploring(false).walk(closest_exit, 0, creep, role, enemies);
                TaskResult::StillWorking(Task::CombatDismantle(id, workplace, members), Some(goal))
            } else {
                warn!("{} no exit found in room {}", creep.name(), creep.pos().room_name());
                TaskResult::Abort
            }
        } else if creep.pos().is_equal_to(workplace) {
            if let Some(structure) = id.resolve().map(|str: Structure| StructureObject::from(str)) {
                if let Some(dismantleable) = structure.as_dismantleable() {
                    let _ = creep.dismantle(dismantleable);
                    let goal = Walker::Exploring(false).walk(structure.pos(), 0, creep, role, enemies);
                    TaskResult::StillWorking(Task::CombatDismantle(id, workplace, members), Some(goal))
                } else {
                    error!("{} structure: {} isn't dismantleable! resolve dismantle request!", creep.name(), id);
                    TaskResult::ResolveRequest(Task::CombatDismantle(id, workplace, members))
                }
            } else {
                error!("{} not found structure: {}, resolve dismantle request!", creep.name(), id);
                TaskResult::ResolveRequest(Task::CombatDismantle(id, workplace, members))
            }
        } else {
            let goal = Walker::Exploring(false).walk(workplace, 0, creep, role, enemies);
            TaskResult::StillWorking(Task::CombatDismantle(id, workplace, members), Some(goal))
        }
    } else {
        //wait for member getting fresh members on the next tick
        TaskResult::Completed
    }
}
