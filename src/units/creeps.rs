use std::collections::{HashMap, HashSet};

use log::{debug, info, warn};
use screeps::{Creep, HasPosition, Position, RoomName, SharedCreepProperties, game};
use serde::{Deserialize, Serialize};

use crate::movement::{Movement, MovementGoal, MovementProfile, PathState};
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::Request;
use crate::units::roles::{Kind, Role};
use crate::units::tasks::{Task, TaskResult};

pub struct CrUnit<'m, 'h, 's> {
    creep: Creep,
    memory: &'m mut CreepMemory,
    home: &'h mut Shelter<'s>,
}

impl CrUnit<'_, '_, '_> {
    fn name(&self) -> String {
        self.creep.name()
    }

    fn pos(&self) -> Position {
        self.creep.pos()
    }

    fn can_move(&self) -> bool {
        self.creep.fatigue() == 0
            && !matches!(self.memory.role.get_movement_profile(&self.creep), MovementProfile::Cargo)
    }

    fn add_request(&mut self, task: Task) {
        if let Ok(request) = Request::try_from(task) {
            info!("{}: {} inserted request: {:?}", self.home.name(), self.name(), request);
            self.home.add_request(request);
        } else {
            warn!("{} can't get room request from task", self.name());
        }
    }

    fn resolve_request(&mut self, task: Task, suicide: bool) {
        if let Ok(request) = Request::try_from(task) {
            self.home.resolve_request(request, self.name());

            if suicide {
                self.memory.respawned = true;
            }
        }
    }

    fn update_request(&mut self, task: Task) {
        if let Ok(mut req) = Request::try_from(task) {
            req.join(Some(self.name()), None);
            self.home.replace_request(req);
        }
    }

    pub fn run_unit(&mut self, _black_list: &HashSet<String>) -> Option<MovementGoal> {
        let task = self
            .memory
            .task
            .take()
            .unwrap_or_else(|| self.memory.role.get_task(&self.creep, self.home));

        match task.run_task(&self.creep, &self.memory.role) {
            TaskResult::StillWorking(task, movement_goal) => {
                self.memory.task = Some(task);
                movement_goal
            }
            TaskResult::RunAnother(task) => {
                debug!("{} immediately run another task: {:?}", self.name(), task);
                match task.run_task(&self.creep, &self.memory.role) {
                    TaskResult::StillWorking(task, movement_goal) => {
                        self.memory.task = Some(task);
                        movement_goal
                    }
                    _ => None,
                }
            }
            TaskResult::ResolveRequest(task, gracefull_suicide) => {
                self.resolve_request(task, gracefull_suicide);
                None
            }
            TaskResult::UpdateRequest(task) => {
                //handy mans don't take requests fromroom memory, so no need to save them
                if !matches!(self.memory.role, Role::HandyMan(_)) {
                    self.update_request(task);
                }
                None
            }
            TaskResult::AddNewRequest(task, another, movement_goal) => {
                self.memory.task = Some(task);
                self.add_request(another);
                movement_goal
            }
            TaskResult::Abort => {
                let task = self.memory.role.get_task(&self.creep, self.home);
                match task.run_task(&self.creep, &self.memory.role) {
                    TaskResult::StillWorking(task, movement_goal) => {
                        self.memory.task = Some(task);
                        movement_goal
                    }
                    _ => None,
                }
            }
            TaskResult::Suicide => {
                self.memory.respawned = true;
                let _ = self.creep.say("☠", false);
                let _ = self.creep.suicide();
                None
            }
            TaskResult::Completed => None,
        }
    }

    pub fn move_to_goal(self, mut goal: Option<MovementGoal>, movement: &mut Movement) {
        if self.can_move() {
            let position = self.pos();
            //creep is not resting and is able to move
            if let Some(mut movement_goal) = goal.take() {
                if movement_goal.is_goal_met(position) {
                    // goal is met! unset the path_state if there is one and idle
                    movement.idle(position, self.creep.into());
                    self.memory.path_state = None;
                } else {
                    let new_path_state = if let Some(mut current_path) = self.memory.path_state.take() {
                        // first call the function that updates the current position
                        // (or the stuck count if we didn't move)
                        if current_path.check_if_moved_and_update_pos(position) {
                            PathState::try_new(position, movement_goal, movement.get_find_route_options())
                        } else if current_path.stuck_threshold_exceed() {
                            debug!("{}, is last step, progress: {}, path.len: {}, stuck.count: {}", self.name(), current_path.path_progress, current_path.path.len(), current_path.stuck_count);
                            movement_goal.avoid_creeps = true;
                            PathState::try_new(position, movement_goal, movement.get_find_route_options())
                        } else if movement_goal.pos != current_path.goal.pos || movement_goal.range < current_path.goal.range {
                            //if goal pos is changed -> find new path
                            PathState::try_new(position, movement_goal, movement.get_find_route_options())
                        } else if movement_goal.repath_needed(&current_path.goal) {
                            if let Some(new_path) = PathState::try_new(position, movement_goal, movement.get_find_route_options()) {
                                //todo prefer longest way if enemies nearby? many enemies? boosted?
                                if new_path.path.len() + 5 < current_path.path.len() {
                                    debug!("{} from: {}, new path + 5: {} shorter then prev: {}, new path: {:?}", self.name(), position, new_path.path.len(), current_path.path.len(), new_path);
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
                    .and_then(|path_state| movement.move_creep(self.creep.into(), path_state));

                    self.memory.path_state = new_path_state;
                }
            } else {
                // no goal, mark as idle!
                movement.idle(position, self.creep.into());
            }
        }
    }

    fn try_respawn(&mut self) {
        if !self.memory.respawned
            && self.creep.ticks_to_live().is_some_and(|ticks| {
                (ticks as usize)
                    < self.memory.role.respawn_timeout(Some(&self.creep)).unwrap_or_default()
            })
        {
            debug!("time to respawn {}", self.creep.name());
            self.memory.respawned = true;
            self.home.add_to_spawn(self.memory.role.clone(), 1);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreepMemory {
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub respawned: bool,
    #[serde(skip)]
    pub task: Option<Task>,
    #[serde(skip)]
    pub path_state: Option<PathState>,
}

impl CreepMemory {
    pub fn new(role: Role) -> Self {
        Self { role, ..Default::default() }
    }

    const fn get_home(&self) -> Option<&RoomName> {
        self.role.get_home()
    }
}

pub fn run_creeps(
    creeps_state: &mut HashMap<String, CreepMemory>,
    homes: &mut HashMap<RoomName, Shelter<'_>>,
    movement: &mut Movement,
    black_list: &HashSet<String>,
) {
    let mut creeps: HashMap<String, Creep> = game::creeps().entries().collect();

    for (name, memory) in creeps_state.iter_mut() {
        let creep = match creeps.remove(name) {
            Some(c) if !c.spawning() => c,
            _ => continue, //gc will clear them
        };

        if let Some(mut unit) = memory
            .get_home()
            .as_ref()
            .and_then(|home_name| homes.get_mut(home_name))
            .map(|home| CrUnit { creep, memory, home })
        {
            unit.try_respawn();
            let goal = unit.run_unit(black_list);
            unit.move_to_goal(goal, movement);
        }
    }
}
