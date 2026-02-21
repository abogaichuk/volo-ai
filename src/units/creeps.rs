use std::collections::{HashMap, HashSet};

use log::{debug, info, warn};
use screeps::{Creep, HasPosition, Position, RoomName, SharedCreepProperties, game};
use serde::{Deserialize, Serialize};

use crate::movement::{Movement, MovementGoal, MovementProfile, PathState};
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::Request;
use crate::units::{move_to_goal_common, tasks::{Task, TaskResult}, roles::{Kind, Role}};

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

    fn resolve_request(&mut self, task: Task) {
        if let Ok(request) = Request::try_from(task) {
            self.home.resolve_request(request, self.name());
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
            TaskResult::ResolveRequest(task) => {
                self.resolve_request(task);
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
        let name = self.name();
        let position = self.pos();
        let can_move = self.can_move();
        let unit = self.creep.into();

        move_to_goal_common(
            name.as_str(),
            position,
            unit,
            goal.take(),
            movement,
            &mut self.memory.path_state,
            can_move,
        );
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
