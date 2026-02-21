use log::{debug, warn};
use screeps::{Creep, HasPosition, Position, SharedCreepProperties};

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::{Task, TaskResult};
use crate::utils::commons::find_walkable_positions_remoted_from;
use crate::utils::constants::{ESCAPE_FROM_EDGE_RANGE, HIDE_TIMEOUT};

pub fn hide(
    position: Position,
    timeout: u32,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    debug!("{} hiding to :{}, timeout: {}", creep.name(), position, timeout);
    if timeout < 1 {
        TaskResult::Abort
    } else if creep.pos().is_equal_to(position) {
        let _ = creep.say("🚬", true); //smokin sign
        TaskResult::StillWorking(Task::Idle(timeout - 1), None)
    } else {
        let _ = creep.say("🤫", false); //quiet face
        let goal = Walker::Exploring(false).walk(position, 0, creep, role, enemies);
        TaskResult::StillWorking(Task::Hide(position, timeout - 1), Some(goal))
    }
}

pub fn escape(position: Position, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    debug!("{} escaping to :{}", creep.name(), position);
    if creep.pos().is_room_edge()
        && position.is_room_edge()
        && creep.pos().room_name() != position.room_name()
    {
        if let Some(remote_from_edge_pos) =
            find_walkable_positions_remoted_from(creep.pos(), ESCAPE_FROM_EDGE_RANGE)
        {
            let _ = creep.say("🤫", false); //quiet face
            let goal = Walker::Exploring(false).walk(remote_from_edge_pos, 0, creep, role, enemies);
            TaskResult::StillWorking(Task::Hide(remote_from_edge_pos, HIDE_TIMEOUT), Some(goal))
        } else {
            warn!("{} no remoted position from edge position found {}", creep.name(), position);
            TaskResult::Abort
        }
    } else {
        let _ = creep.say("🏃", true); //runnin sign

        let goal = Walker::Exploring(false).walk(position, 0, creep, role, enemies);
        TaskResult::StillWorking(Task::Escape(position), Some(goal))
    }
}
