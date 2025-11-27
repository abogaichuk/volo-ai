use serde::{Serialize, Deserialize};
use screeps::{game::{self, map::RoomStatus}, Creep, HasPosition, Part, Position, RoomName};
use std::fmt;
use arrayvec::ArrayVec;
use crate::{
    movement::{MovementProfile, walker::Walker}, rooms::shelter::Shelter, commons::get_random
};
use super::{Kind, Task};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scout {
    pub(crate) home: Option<RoomName>,
    pub(crate) target: Option<Position>
}

impl fmt::Debug for Scout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Kind for Scout {
    fn respawn_timeout(&self, _: Option<&Creep>) -> Option<u32> { Some(1) }
    
    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::SwampFiveToOne
    }

    fn body(&self, _: u32) -> ArrayVec<[Part; 50]> {
        [Part::Move].into_iter().collect()
    }

    fn get_task(&self, creep: &Creep, _: &mut Shelter) -> Task {
        self.target
            .map(|target| {
                if creep.pos().room_name() == target.room_name() {
                    Task::Provoke(5, 10)
                } else {
                    Task::MoveMe(target.room_name(), Walker::Exploring(false))
                }
            })
            .or_else(|| {
                let values: Vec<RoomName> = game::map::describe_exits(creep.pos().room_name()).values().collect();
                let index = get_random(0, values.len());

                values.get(index)
                    .filter(|room_name| game::map::get_room_status(**room_name)
                        .is_some_and(|room_status| room_status.status() == RoomStatus::Normal))
                    .map(|room_name| Task::MoveMe(*room_name, Walker::Exploring(false)))
            })
            .unwrap_or_default()
    }
}

impl Scout {
    pub fn new(home: Option<RoomName>, target: Option<Position>) -> Self {
        Self { home, target }
    }
}