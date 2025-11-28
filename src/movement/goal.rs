use log::*;

use screeps::{
    game::map, local::{Position, RoomName}, pathfinder::SearchResults, RoomXY
};
use crate::{
    movement::{callback::PathOptions, MovementProfile},
    utils::commons::is_cpu_on_low,
    utils::constants::*,
};
use std::collections::HashSet;
use serde::{Serialize, Deserialize};

// struct for specifying where a creep wants to move and the options the pathfinder
// will need to know to get them there
#[derive(Eq, PartialEq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct MovementGoal {
    pub pos: Position,
    pub range: u32,
    pub profile: MovementProfile,
    pub avoid_creeps: bool,
    pub flee: bool,
    pub danger_zones: Option<(RoomName, Vec<RoomXY>)>,
}

pub struct MovementGoalBuilder {
    pos: Position,
    range: u32,
    profile: MovementProfile,
    avoid_creeps: bool,
    flee: bool,
    danger_zones: Option<(RoomName, Vec<RoomXY>)>
}

impl MovementGoalBuilder {
    pub fn new(pos: Position) -> Self {
        Self {
            pos,
            range: 0,
            profile: MovementProfile::RoadsOneToTwo,
            avoid_creeps: false,
            flee: false,
            danger_zones: None
        }
    }

    pub fn range(mut self, range: u32) -> MovementGoalBuilder {
        self.range = range;
        self
    }

    pub fn profile(mut self, profile: MovementProfile) -> MovementGoalBuilder {
        self.profile = profile;
        self
    }

    pub fn danger_zones(mut self, danger_zones: Option<(RoomName, Vec<RoomXY>)>) -> MovementGoalBuilder {
        self.danger_zones = danger_zones;
        self
    }

    pub fn avoid_creeps(mut self, avoid_creeps: bool) -> MovementGoalBuilder {
        self.avoid_creeps = avoid_creeps;
        self
    }

    pub fn flee(mut self) -> MovementGoalBuilder {
        self.flee = true;
        self
    }

    // If we can get away with not consuming the Builder here, that is an
    // advantage. It means we can use the FooBuilder as a template for constructing
    // many Foos.
    pub fn build(self) -> MovementGoal {
        MovementGoal {
            pos: self.pos,
            range: self.range,
            profile: self.profile,
            avoid_creeps: self.avoid_creeps,
            flee: self.flee,
            danger_zones: self.danger_zones
        }
    }
}

impl MovementGoal {

    pub fn repath_needed(&self, previous_goal: &MovementGoal) -> bool {
        //check for avoid creep is excessive here, because of stuck count handle this
        self.danger_zones != previous_goal.danger_zones || self.profile != previous_goal.profile
    }

    pub fn is_goal_met(&self, position: Position) -> bool {
        if self.flee {
            position.get_range_to(self.pos) >= self.range
        } else {
            position.get_range_to(self.pos) <= self.range
        }
    }

    pub fn find_path(
        &self,
        from_position: Position,
        route_options: map::FindRouteOptions<impl FnMut(RoomName, RoomName) -> f64>) -> Option<SearchResults>
    {
        //todo increase max rooms for flee case?
        let (max_rooms, allowed_rooms) = if from_position.room_name() == self.pos.room_name() {
            (1, HashSet::new())
        } else {
            let allowed_rooms = self.find_route(from_position.room_name(), route_options);
            (MAX_ROOMS, allowed_rooms)
        };

        if is_cpu_on_low() && max_rooms > 1 {
            None
        } else {
            let options = PathOptions {
                from: from_position,
                // flee: self.flee,
                avoid_creeps: self.avoid_creeps,
                danger_zones: self.danger_zones.clone(),
                allowed_rooms
            };
        
            Some(screeps::pathfinder::search(
                from_position,
                self.pos,
                self.range,
                self.profile.search_options(options, self.flee, max_rooms)
            ))
        }
    }

    pub fn find_route(
        &self,
        from_room: RoomName,
        route_options: map::FindRouteOptions<impl FnMut(RoomName, RoomName) -> f64>) -> HashSet<RoomName>
    {
        let mut allowed_rooms = match map::find_route(
            from_room,
            self.pos.room_name(),
            Some(route_options)) {
                Ok(steps) => {
                    steps.into_iter()
                        .map(|step| step.room)
                        .collect()
                }
                Err(e) => {
                    warn!("can't find high level route: {:?}", e);
                    HashSet::new()
                }
        };
        allowed_rooms.insert(from_room);
        allowed_rooms.insert(self.pos.room_name());
    
        debug!("find route from: {}, to: {}, allowed_rooms: {:?}", from_room, self.pos.room_name(), allowed_rooms);
        allowed_rooms
    }

    // fn find_route_needed(&self, from: RoomName) -> bool {
    //     if from == self.pos.room_name() {
    //         false
    //     } else {
    //         !map::describe_exits(from)
    //             .values()
    //             .contains(&self.pos.room_name())
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use screeps::{RoomCoordinate, RoomXY};

    #[test]
    fn danger_zones_equality_test() {
        let zones1 = vec![RoomXY::new(RoomCoordinate(1), RoomCoordinate(5))];
        let zones2 = vec![RoomXY::new(RoomCoordinate(1), RoomCoordinate(5))];

        assert!(zones1 == zones2)
    }
}

