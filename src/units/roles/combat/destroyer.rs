use std::fmt;

use arrayvec::ArrayVec;
use screeps::{Creep, Part, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, can_scale, pvp_parts_priority};
use crate::movement::MovementProfile;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Destroyer {
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Destroyer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {}", home) } else { write!(f, "") }
    }
}

impl Destroyer {
    pub fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Destroyer {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [
            Part::Heal,
            Part::Heal,
            Part::Heal,
            Part::Tough,
            Part::Tough,
            Part::Move,
            Part::Move,
            Part::RangedAttack,
            Part::RangedAttack,
        ];

        let mut body = [
            Part::Tough,
            Part::RangedAttack,
            Part::RangedAttack,
            Part::RangedAttack,
            Part::RangedAttack,
        ]
        .into_iter()
        .collect::<ArrayVec<[Part; 50]>>();

        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().cloned());
        }

        body.sort_by_key(|a| pvp_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() > creep.hits_max() - creep.hits_max() / 5 {
            MovementProfile::PlainsOneToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }
}
