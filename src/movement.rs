use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::Rc;

use log::*;
use screeps::action_error_codes::SayErrorCode;
use screeps::constants::Direction;
use screeps::game::map::FindRouteOptions;
use screeps::local::Position;
use screeps::pathfinder::{
    MultiRoomCostResult, SearchGoal, SearchOptions, SearchResults, SingleRoomCostResult,
};
use screeps::visual::{LineDrawStyle, PolyStyle, RoomVisual};
use screeps::{
    CostMatrix, Creep, FindPathOptions, HasPosition, Path, PowerCreep, RoomName, RoomPosition,
    RoomXY, Step,
};
use serde::{Deserialize, Serialize};

use crate::commons::{capture_room_parts, get_room_regex, is_highway, is_skr};
use crate::movement::callback::{PathOptions, SingleRoomCallback};
use crate::utils::constants::{HEURISTIC_WEIGHT, MAX_OPS};

pub mod callback;
mod goal;
mod path_state;
pub mod walker;

pub use goal::{MovementGoal, MovementGoalBuilder};
pub use path_state::PathState;

type MultiRoomSearchOptions = SearchOptions<Box<dyn FnMut(RoomName) -> MultiRoomCostResult>>;

// enum for the different speeds available to creeps
#[derive(Eq, PartialEq, Hash, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum MovementProfile {
    // can move at full speed on swamp (either 5:1 move parts ratio, or
    // all parts are move/empty carry)
    SwampFiveToOne,
    // can move at full speed on plains (1:1 move ratio)
    #[default]
    PlainsOneToOne,
    // can only move once per tick on roads, weight them appropriately
    RoadsOneToTwo,
    //can't move byself
    Cargo,
}

impl MovementProfile {
    pub fn search_options(
        &self,
        options: PathOptions,
        flee: bool,
        max_rooms: u8,
    ) -> Option<MultiRoomSearchOptions> {
        match self {
            MovementProfile::SwampFiveToOne => Some(
                SearchOptions::new(callback::prefer_swamp_callback(options))
                    .max_ops(MAX_OPS)
                    .max_rooms(max_rooms)
                    .swamp_cost(1)
                    .flee(flee)
                    .heuristic_weight(HEURISTIC_WEIGHT),
            ),
            MovementProfile::PlainsOneToOne => Some(
                SearchOptions::new(callback::prefer_plain_callback(options))
                    .max_ops(MAX_OPS)
                    .max_rooms(max_rooms)
                    .swamp_cost(4)
                    .flee(flee)
                    .heuristic_weight(HEURISTIC_WEIGHT),
            ),
            MovementProfile::RoadsOneToTwo => Some(
                SearchOptions::new(callback::prefer_roads_callback(options))
                    .max_ops(MAX_OPS)
                    .max_rooms(max_rooms)
                    .plain_cost(2)
                    .swamp_cost(10)
                    .flee(flee)
                    .heuristic_weight(HEURISTIC_WEIGHT),
            ),
            MovementProfile::Cargo => None,
        }
    }
}

pub struct Movement {
    idle_creeps: HashMap<Position, MovableUnit>,
    moving_creeps: HashMap<Position, Direction>,
    //store Fn instead of FnMut because Rc gives a cheap clones and shared (&) access
    room_callback: Rc<dyn Fn(RoomName, RoomName) -> f64 + 'static>,
}

impl Movement {
    pub fn new(avoid_rooms: &HashMap<RoomName, u32>, owned: Vec<RoomName>) -> Self {
        let callback = find_route_callback(avoid_rooms, owned);
        Self {
            idle_creeps: HashMap::new(),
            moving_creeps: HashMap::new(),
            room_callback: Rc::new(callback),
        }
    }

    pub fn get_find_route_options(
        &self,
    ) -> FindRouteOptions<Box<dyn FnMut(RoomName, RoomName) -> f64 + 'static>> {
        let cb = Rc::clone(&self.room_callback);

        // Wrap into a Box<dyn FnMut..> to satisfy the generic parameter F.
        let route_cb: Box<dyn FnMut(RoomName, RoomName) -> f64 + 'static> =
            Box::new(move |to, from| (cb)(to, from));

        FindRouteOptions::new().room_callback(route_cb)
    }

    pub fn idle(&mut self, position: Position, unit: MovableUnit) {
        self.idle_creeps.insert(position, unit);
    }

    pub fn swap_move(&self) {
        // look for idle creeps where we actively have creeps saying they intend to move
        for (dest_pos, moving_direction) in self.moving_creeps.iter() {
            if let Some(creep) = self.idle_creeps.get(dest_pos) {
                let backward_direction = -*moving_direction;
                creep.move_direction(backward_direction);
                let _ = creep.say(format!("{}", backward_direction).as_str(), true);
            }
        }
    }

    pub fn move_creep(
        &mut self,
        unit: MovableUnit,
        mut path_state: PathState,
    ) -> Option<PathState> {
        let current_position = unit.position();

        if cfg!(feature = "path-visuals") {
            let mut points = vec![];
            let mut cursor_pos = current_position;
            for step in path_state.path[path_state.path_progress..].iter() {
                cursor_pos = cursor_pos + *step;
                if cursor_pos.room_name() != current_position.room_name() {
                    break;
                }
                points.push((cursor_pos.x().u8() as f32, cursor_pos.y().u8() as f32));
            }
            RoomVisual::new(Some(current_position.room_name())).poly(
                points,
                Some(
                    PolyStyle::default()
                        .fill("transparent")
                        .stroke("#f00")
                        .line_style(LineDrawStyle::Dashed)
                        .stroke_width(0.15)
                        .opacity(0.5),
                ),
            );
        }

        // debug!("creep: {}, progress: {} path: {:?} ", creep.name(),
        // path_state.path_progress, path_state.path);
        match path_state.path.get(path_state.path_progress) {
            Some(direction) => {
                // do the actual move in the intended direction
                unit.move_direction(*direction);
                // set next_direction so we can detect if this worked next tick
                path_state.next_direction = *direction;
                // insert a key of the position the creep intends to move to,
                // and a value of the direction this creep is moving (so a creep
                // at the target position can infer which direction they should move to swap)
                // moving_creeps.insert(current_position + *direction, *direction);
                self.moving_creeps.insert(current_position + *direction, *direction);
                Some(path_state)
            }
            None => None,
        }
    }
}

pub fn find_path_in_room<C>(from: RoomPosition, to: RoomXY, range: u32, callback: C) -> Vec<Step>
where
    C: FnMut(RoomName, CostMatrix) -> SingleRoomCostResult,
{
    let fpo = FindPathOptions::<SingleRoomCallback, SingleRoomCostResult>::new()
        .cost_callback(callback)
        .range(range)
        .ignore_creeps(true)
        .plain_cost(3)
        .swamp_cost(4);

    match from.find_path_to_xy(to.x, to.y, Some(fpo)) {
        Path::Vectorized(v) => v,
        Path::Serialized(_) => Vec::new(), //todo deserialize
    }
}

pub fn find_many_in_room<C>(
    from: Position,
    goals: impl Iterator<Item = SearchGoal>,
    callback: C,
) -> SearchResults
where
    C: FnMut(RoomName) -> MultiRoomCostResult,
{
    let options = SearchOptions::new(callback)
        .max_ops(2000) //default value, could be reduced
        .max_rooms(1) //default value, could be reduced
        .plain_cost(3)
        .swamp_cost(4);

    screeps::pathfinder::search_many(from, goals, Some(options))
}

pub fn find_path<C>(from: Position, goal: Position, range: u32, callback: C) -> SearchResults
where
    C: FnMut(RoomName) -> MultiRoomCostResult,
{
    let options = SearchOptions::new(callback)
        .max_ops(2000) //default value, could be reduced
        .max_rooms(16) //default value, could be reduced
        .plain_cost(3)
        .swamp_cost(4);

    screeps::pathfinder::search(from, goal, range, Some(options))
}

pub fn find_many<C>(
    from: Position,
    goals: impl Iterator<Item = SearchGoal>,
    callback: C,
) -> SearchResults
where
    C: FnMut(RoomName) -> MultiRoomCostResult,
{
    let options = SearchOptions::new(callback)
        .max_ops(2000) //default value, could be reduced
        .max_rooms(16) //default value, could be reduced
        .plain_cost(3)
        .swamp_cost(4);

    screeps::pathfinder::search_many(from, goals, Some(options))
}

fn find_route_callback(
    avoid_rooms: &HashMap<RoomName, u32>,
    owned: Vec<RoomName>,
) -> impl Fn(RoomName, RoomName) -> f64 + 'static {
    let avoid_keys: HashSet<RoomName> = avoid_rooms.keys().cloned().collect();
    let re = get_room_regex();

    move |to_room: RoomName, _from_room: RoomName| {
        if avoid_keys.contains(&to_room) {
            f64::MAX
        } else if let Some((f_mod, s_mod)) = capture_room_parts(&re, to_room) {
            debug!(
                "find_route callback -> room: {}, f_cap_mod: {}, s_cap_mod: {}",
                to_room, f_mod, s_mod
            );
            if is_highway(f_mod, s_mod) || owned.contains(&to_room) {
                1.
            } else if is_skr(f_mod, s_mod) {
                1.3
            } else {
                1.15
            }
        } else {
            f64::MAX
        }
    }
}

pub enum MovableUnit {
    Creep(Creep),
    Power(PowerCreep),
}

impl MovableUnit {
    pub fn position(&self) -> Position {
        match self {
            MovableUnit::Creep(c) => c.pos(),
            MovableUnit::Power(pc) => pc.pos(),
        }
    }

    pub fn move_direction(&self, dir: Direction) {
        match self {
            MovableUnit::Creep(c) => {
                let _ = Creep::move_direction(c, dir);
            }
            MovableUnit::Power(pc) => {
                let _ = PowerCreep::move_direction(pc, dir);
            }
        }
    }

    pub fn say(&self, message: &str, public: bool) -> Result<(), SayErrorCode> {
        match self {
            MovableUnit::Creep(c) => c.say(message, public),
            MovableUnit::Power(pc) => pc.say(message, public),
        }
    }
}

impl From<Creep> for MovableUnit {
    fn from(c: Creep) -> Self {
        MovableUnit::Creep(c)
    }
}

impl From<PowerCreep> for MovableUnit {
    fn from(pc: PowerCreep) -> Self {
        MovableUnit::Power(pc)
    }
}
