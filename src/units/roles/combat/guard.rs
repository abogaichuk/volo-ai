use std::collections::HashMap;
use std::fmt;

use arrayvec::ArrayVec;
use screeps::{Creep, Part, ResourceType, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, pvp_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guard {
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Guard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl Guard {
    pub const fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Guard {
    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() > creep.hits_max() - creep.hits_max() / 5 {
            MovementProfile::PlainsOneToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Attack, Part::Move, Part::Move, Part::Heal];

        let mut body = [
            Part::Attack,
            Part::Move,
            Part::Attack,
            Part::Move,
            Part::Attack,
            Part::Move,
            Part::Attack,
            Part::Move,
            Part::Attack,
            Part::Move,
        ]
        .into_iter()
        .collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| pvp_parts_priority(*a));
        body
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        if creep.ticks_to_live().is_some_and(|tick| tick > 1400) {
            [(Part::Attack, [ResourceType::CatalyzedUtriumAcid, ResourceType::UtriumAcid])].into()
        } else {
            HashMap::new()
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        home.get_available_boost(creep, self.boosts(creep))
            .map_or_else(
                || Task::DefendHome,
                |(id, body_part)| Task::Boost(id, u32::try_from(
                    creep.body().iter().filter(|bp| bp.part() == body_part).count()).ok()
                ))
    }
}
