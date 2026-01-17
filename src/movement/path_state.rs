use log::{debug, warn};
use screeps::RoomName;
use screeps::constants::Direction;
use screeps::local::Position;
use serde::{Deserialize, Serialize};

use crate::movement::{FindRouteOptions, MovementGoal};
use crate::utils::constants::STUCK_REPATH_THRESHOLD;

// struct for tracking the current state of a moving creep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathState {
    // track the goal this state moves towards - we'll confirm the creep
    // hasn't registered a new goal before using this cached state
    pub goal: MovementGoal,
    pub stuck_count: u8,
    pub last_position: Position,
    pub next_direction: Direction,
    pub path: Vec<Direction>,
    pub path_progress: usize,
}

impl PathState {
    pub fn try_new(
        from: Position,
        goal: MovementGoal,
        route_options: FindRouteOptions<impl FnMut(RoomName, RoomName) -> f64>,
    ) -> Option<Self> {
        if from.is_near_to(goal.pos) && goal.range == 0 {
            let next_direction = from.get_direction_to(goal.pos).unwrap_or(Direction::Top);
            Some(PathState {
                goal,
                stuck_count: 0,
                last_position: from,
                next_direction,
                path: vec![next_direction],
                path_progress: 0,
            })
        } else if let Some(search_result) = goal.find_path(from, route_options) {
            if search_result.incomplete() {
                warn!(
                    "incomplete search! {} {} {} {}",
                    search_result.ops(),
                    search_result.cost(),
                    from,
                    goal.pos
                );
            }

            let mut cursor = from;
            let mut steps = Vec::with_capacity(search_result.path().len());

            for pos in search_result.path() {
                // skip storing this step if it's just a room boundary change
                // that'll happen automatically thanks to the edge tile's swap-every-tick
                if pos.room_name() == cursor.room_name() {
                    if let Some(v) = pos.get_direction_to(cursor) {
                        // store the inverse of the direction to cursor_pos,
                        // since it's earlier in the path
                        let v = -v;
                        steps.push(v);
                    } else {
                        warn!("direction failure?");
                        break;
                    }
                }
                cursor = pos;
            }

            Some(PathState {
                goal,
                stuck_count: 0,
                last_position: from,
                // in the rare case we got a zero-step incomplete path, just
                // mark top as the direction we're moving; the path will just fail next tick
                next_direction: *steps.first().unwrap_or(&Direction::Top),
                path: steps,
                path_progress: 0,
            })
        } else {
            debug!("can't find a path from: {}, goal: {:?}", from, goal);
            None
        }
    }

    pub const fn stuck_threshold_exceed(&self) -> bool {
        self.stuck_count >= STUCK_REPATH_THRESHOLD
    }

    pub const fn is_last_step(&self) -> bool {
        self.path_progress + 1 == self.path.len()
    }

    pub fn check_if_moved_and_update_pos(&mut self, current_position: Position) -> bool {
        // first we'll check if the creep actually moved as we intended last tick,
        // incrementing the path_progress if so (and incrementing the stuck_count if
        // not)
        if current_position == (self.last_position + self.next_direction)
            || passed_edge(current_position, self.last_position)
        {
            // we've moved as intended (yay); let's update the last good position..
            self.last_position = current_position;
            // ..and bump the cursor for the next move..
            self.path_progress += 1;
            // ..and reset the stuck count
            self.stuck_count = 0;
            false
        } else if current_position == self.last_position {
            // didn't move, simply increment the stuck counter
            self.stuck_count += 1;
            false
        } else {
            // we're not in the right spot. If we're in a different position than we were
            // last tick, something weird is going on (possibly stuck on an exit tile or
            // portal) - we want to repath in this case, so send the stuck count
            // way up to trigger repathing
            debug!(
                "weird position:{:?}, last_pos: {} stuck_count == MAX!",
                current_position, self.last_position
            );
            true
            // self.stuck_count = u8::MAX;
        }
    }
}

fn passed_edge(current_position: Position, last_position: Position) -> bool {
    // info!("current_position: {}, last_position: {}", current_position,
    // last_position);
    current_position.is_room_edge() && current_position.room_name() != last_position.room_name()
}
