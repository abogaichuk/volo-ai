use log::*;
use screeps::{
    ConstructionSite, Creep, HasPosition, ObjectId, Position, ResourceType, Part,
    SharedCreepProperties
};
use crate::{
    units::{Task, TaskResult, roles::Role, with_parts},
    movement::walker::Walker,
    utils::constants::LONG_RANGE_ACTION
};

pub fn build(id: Option<ObjectId<ConstructionSite>>, pos: Position, creep: &Creep, role: &Role, hostiles: Vec<Creep>) -> TaskResult {
    let attackers = with_parts(hostiles, vec![Part::Attack, Part::RangedAttack]);
    if creep.store().get_used_capacity(Some(ResourceType::Energy)) == 0 {
        TaskResult::Abort
    } else if creep.pos().room_name() != pos.room_name() {
        TaskResult::RunAnother(Task::MoveMe(pos.room_name(), Walker::Reinforcing))
    } else if creep.pos().is_room_edge() {
        if creep.pos().in_range_to(pos, LONG_RANGE_ACTION) {
            let goal = Walker::Reinforcing.walk(pos, 0, creep, role, attackers);
            TaskResult::StillWorking(Task::Build(id, pos), Some(goal))
        } else {
            let goal = Walker::Reinforcing.walk(pos, LONG_RANGE_ACTION, creep, role, attackers);
            TaskResult::StillWorking(Task::Build(id, pos), Some(goal))
        }
    } else if let Some(cs) = id.and_then(|id| id.resolve()) {
        if creep.pos().in_range_to(pos, LONG_RANGE_ACTION) {
            let _ = creep.say("🛠︎", false);
            let _ = creep.build(&cs);
            TaskResult::StillWorking(Task::Build(id, pos), None)
        } else {
            let goal = Walker::Reinforcing.walk(pos, LONG_RANGE_ACTION, creep, role, attackers);
            TaskResult::StillWorking(Task::Build(id, pos), Some(goal))
        }
    } else {
        warn!("{} no cs at pos:{} found in a room {}", creep.name(), pos, creep.pos().room_name());
        TaskResult::ResolveRequest(Task::Build(id, pos), false)
    }
}