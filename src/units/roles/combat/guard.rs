use serde::{Serialize, Deserialize};
use screeps::{Part, ResourceType, RoomName, Creep};
use std::{fmt, collections::HashMap};
use arrayvec::ArrayVec;
use crate::{movement::MovementProfile, rooms::shelter::Shelter};
use super::{Kind, Task, can_scale, pvp_parts_priority};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Guard {
    pub(crate) home: Option<RoomName>
}

impl fmt::Debug for Guard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Guard {
    pub fn new(home: Option<RoomName>) -> Self {
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

        let mut body = [Part::Attack, Part::Move, Part::Attack, Part::Move, Part::Attack,
            Part::Move, Part::Attack, Part::Move, Part::Attack, Part::Move]
            .into_iter()
            .collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().cloned());
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
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter()
                    .filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, Some(parts_number as u32))
            })
            .unwrap_or_else(|| Task::DefendHome)
    }
}