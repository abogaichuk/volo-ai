use std::collections::HashMap;
use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{Part, ResourceType, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, pvp_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fighter {
    target: Option<RoomName>,
    pub(crate) home: Option<RoomName>,
    #[serde(default)]
    boost: bool,
}

impl fmt::Debug for Fighter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}, ", home)?;
        }
        if let Some(target) = &self.target {
            write!(f, "target: {}", target)?;
        }
        write!(f, "")
    }
}

impl Fighter {
    pub fn new(target: RoomName, home: RoomName, boost: bool) -> Self {
        Self { target: Some(target), home: Some(home), boost }
    }
}

impl Kind for Fighter {
    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() < creep.hits_max() {
            MovementProfile::RoadsOneToTwo
        } else {
            MovementProfile::PlainsOneToOne
        }
    }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let (basic_parts, scale_parts) = if self.boost {
            (
                [
                    Part::RangedAttack,
                    Part::RangedAttack,
                    Part::RangedAttack,
                    Part::RangedAttack,
                    Part::RangedAttack,
                ],
                [
                    Part::Heal,
                    Part::Heal,
                    Part::Tough,
                    Part::Tough,
                    Part::Move,
                    Part::Move,
                    Part::RangedAttack,
                    Part::RangedAttack,
                    Part::RangedAttack,
                ],
            )
        } else {
            (
                [Part::Move, Part::Move, Part::Move, Part::Move, Part::Move],
                [
                    Part::Tough,
                    Part::RangedAttack,
                    Part::RangedAttack,
                    Part::Move,
                    Part::Move,
                    Part::Move,
                    Part::Move,
                    Part::Heal,
                    Part::Heal,
                ],
            )
        };

        let mut body = basic_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().cloned());
        }

        body.sort_by_key(|a| pvp_parts_priority(*a));
        body
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        //todo change ticks to boost here when labs carry requests will be fixed
        if self.boost && creep.ticks_to_live().is_some_and(|tick| tick > 1000) {
            [
                (
                    Part::Move,
                    [ResourceType::CatalyzedZynthiumAlkalide, ResourceType::ZynthiumAlkalide],
                ),
                (
                    Part::RangedAttack,
                    [ResourceType::CatalyzedKeaniumAlkalide, ResourceType::KeaniumAlkalide],
                ),
                (
                    Part::Heal,
                    [ResourceType::CatalyzedLemergiumAlkalide, ResourceType::LemergiumAlkalide],
                ),
                (
                    Part::Tough,
                    [ResourceType::CatalyzedGhodiumAlkalide, ResourceType::GhodiumAlkalide],
                ),
            ]
            .into()
        } else {
            HashMap::new()
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        home.get_available_boost(creep, self.boosts(creep))
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter().filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, Some(parts_number as u32))
            })
            .or_else(|| self.target.map(|target| Task::Protect(target, None)))
            .unwrap_or_default()
    }
}
