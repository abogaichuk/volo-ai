use log::*;
use screeps::action_error_codes::CreepRepairErrorCode;
use screeps::{
    Creep, HasPosition, ObjectId, Part, Position, ResourceType, SharedCreepProperties, Structure,
    StructureObject,
};

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::{Task, TaskResult, with_parts};
use crate::utils::constants::LONG_RANGE_ACTION;

pub fn repair(
    id: ObjectId<Structure>,
    pos: Position,
    times: u8,
    creep: &Creep,
    role: &Role,
    hostiles: Vec<Creep>,
) -> TaskResult {
    //todo add flee
    let attackers = with_parts(hostiles, vec![Part::Attack, Part::RangedAttack]);
    if creep.store().get_used_capacity(Some(ResourceType::Energy)) == 0 {
        TaskResult::UpdateRequest(Task::Repair(id, pos, times))
    } else if !creep
        .body()
        .iter()
        .any(|body_part| body_part.hits() > 0 && body_part.part() == Part::Work)
    {
        TaskResult::Abort
    } else if creep.pos().room_name() != pos.room_name() {
        TaskResult::RunAnother(Task::MoveMe(pos.room_name(), Walker::Reinforcing))
    } else if creep.pos().is_room_edge() {
        if creep.pos().in_range_to(pos, LONG_RANGE_ACTION) {
            let goal = Walker::Reinforcing.walk(pos, 0, creep, role, attackers);
            TaskResult::StillWorking(Task::Repair(id, pos, times), Some(goal))
        } else {
            let goal = Walker::Reinforcing.walk(pos, LONG_RANGE_ACTION, creep, role, attackers);
            TaskResult::StillWorking(Task::Repair(id, pos, times), Some(goal))
        }
    } else if let Some(structure) = id.resolve().map(|str: Structure| StructureObject::from(str)) {
        if creep.pos().in_range_to(pos, LONG_RANGE_ACTION) {
            if let Some(repairable) = structure.as_repairable()
                && repairable.hits() < repairable.hits_max()
            {
                let _ = creep.say("🛠︎", false);
                match creep.repair(repairable) {
                    Ok(_) => {
                        let times = times - 1;
                        if times > 1 {
                            TaskResult::StillWorking(Task::Repair(id, pos, times - 1), None)
                        } else {
                            TaskResult::ResolveRequest(Task::Repair(id, pos, times - 1), false)
                        }
                    }
                    Err(err) => match err {
                        CreepRepairErrorCode::NotEnoughResources
                        | CreepRepairErrorCode::NotInRange => TaskResult::Abort,
                        _ => {
                            error!(
                                "creep: {} can't repair: {}, error: {:?}",
                                creep.name(),
                                id,
                                err
                            );
                            TaskResult::ResolveRequest(Task::Repair(id, pos, times), false)
                        }
                    },
                }
            } else {
                warn!("{} not repairable structure: {}", creep.name(), id);
                TaskResult::ResolveRequest(Task::Repair(id, pos, times), false)
            }
        } else {
            let goal = Walker::Reinforcing.walk(pos, LONG_RANGE_ACTION, creep, role, attackers);
            TaskResult::StillWorking(Task::Repair(id, pos, times), Some(goal))
        }
    } else {
        warn!("{} repair error, no structure found! {}", creep.name(), id);
        TaskResult::ResolveRequest(Task::Repair(id, pos, times), false)
    }
}
