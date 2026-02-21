use log::debug;

use screeps::{Creep, Part, Position, SOURCE_KEEPER_USERNAME};
use thiserror::Error;

use self::roles::Role;
use self::tasks::{Task, TaskResult};
use crate::commons::has_part;
use crate::movement::{MovableUnit, Movement, MovementGoal, PathState};

pub mod creeps;
pub mod power_creep;
pub mod roles;
pub mod tasks;
mod actions;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum UnitError {
    #[error("creep home room is not set")]
    HomeRoomIsNotSet,
}

fn with_parts(enemies: Vec<Creep>, parts: Vec<Part>) -> Vec<Creep> {
    enemies.into_iter().filter(|creep| has_part(&parts, creep, true)).collect()
}

fn need_escape(enemies: &[Creep]) -> bool {
    enemies.iter().any(|hostile| {
        hostile.owner().username() != SOURCE_KEEPER_USERNAME
            && has_part(&[Part::RangedAttack, Part::Attack], hostile, true)
    })
}

fn move_to_goal_common(
    name: &str,
    position: Position,
    unit: MovableUnit,
    goal: Option<MovementGoal>,
    movement: &mut Movement,
    path_state: &mut Option<PathState>,
    can_move: bool,
) {
    if !can_move {
        return;
    }

    if let Some(mut movement_goal) = goal {
        if movement_goal.is_goal_met(position) {
            // goal is met! unset the path_state if there is one and idle
            movement.idle(position, unit);
            *path_state = None;
        } else {
            let new_path_state = if let Some(mut current_path) = path_state.take() {
                // first call the function that updates the current position
                // (or the stuck count if we didn't move)
                if current_path.check_if_moved_and_update_pos(position) {
                    PathState::try_new(position, movement_goal, movement.get_find_route_options())
                } else if current_path.stuck_threshold_exceed() {
                    debug!(
                        "{name}, is last step, progress: {}, path.len: {}, stuck.count: {}",
                        current_path.path_progress,
                        current_path.path.len(),
                        current_path.stuck_count
                    );
                    movement_goal.avoid_creeps = true;
                    PathState::try_new(position, movement_goal, movement.get_find_route_options())
                } else if movement_goal.pos != current_path.goal.pos
                    || movement_goal.range < current_path.goal.range
                {
                    //if goal pos is changed -> find new path
                    PathState::try_new(position, movement_goal, movement.get_find_route_options())
                } else if movement_goal.repath_needed(&current_path.goal) {
                    if let Some(new_path) =
                        PathState::try_new(position, movement_goal, movement.get_find_route_options())
                    {
                        //todo prefer longest way if enemies nearby? many enemies? boosted?
                        if new_path.path.len() + 5 < current_path.path.len() {
                            debug!(
                                "{name} from: {position}, new path + 5: {} shorter then prev: {}, new path: {:?}",
                                new_path.path.len(),
                                current_path.path.len(),
                                new_path
                            );
                            Some(new_path)
                        } else {
                            Some(current_path)
                        }
                    } else {
                        Some(current_path)
                    }
                } else {
                    //if nothing is changed -> use current path
                    Some(current_path)
                }
            } else {
                PathState::try_new(position, movement_goal, movement.get_find_route_options())
            }
            .and_then(|path_state| movement.move_creep(unit, path_state));

            *path_state = new_path_state;
        }
    } else {
        // no goal, mark as idle!
        movement.idle(position, unit);
    }
}
