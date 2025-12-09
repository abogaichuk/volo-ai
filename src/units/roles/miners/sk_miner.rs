use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{HasId, HasPosition, Part, Position, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SKMiner {
    pub(crate) workplace: Option<Position>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for SKMiner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {home}, ")?;
        }
        if let Some(workplace) = &self.workplace {
            write!(f, "workplace: {workplace}")?;
        }
        write!(f, "")
    }
}

impl SKMiner {
    pub const fn new(workplace: Option<Position>, home: Option<RoomName>) -> Self {
        Self { workplace, home }
    }
}

impl Kind for SKMiner {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Work, Part::Work, Part::Move];

        let mut body = [Part::Carry].into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 20) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::RoadsOneToTwo
    }

    fn respawn_timeout(&self, creep: Option<&Creep>) -> Option<u32> {
        creep.map(|c| c.body().len() as u32 * 3).or(Some(0))
    }

    fn get_task(&self, _: &Creep, home: &mut Shelter) -> Task {
        self.workplace
            .and_then(|workplace| {
                home.all_sources()
                    .find(|s| workplace.is_near_to(s.pos()))
                    .map(|s| Task::HarvestEnergyForever(workplace, s.id()))
            })
            .unwrap_or_default()
    }
}
