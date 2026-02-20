use std::{fmt, collections::HashMap};

use arrayvec::ArrayVec;
use screeps::{Part, ResourceType, RoomName, objects::Creep, SharedCreepProperties};
use serde::{Deserialize, Serialize};

use super::Kind;
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::Request;
use crate::units::roles::{Role, can_scale, pvp_parts_priority};
use crate::units::tasks::Task;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComHealer {
    pub(crate) squad_id: Option<String>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for ComHealer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {home}, ")?;
        }
        if let Some(squad_id) = &self.squad_id {
            write!(f, "squad_id: {squad_id}")?;
        }
        write!(f, "")
    }
}

impl ComHealer {
    pub const fn new(squad_id: Option<String>, home: Option<RoomName>) -> Self {
        Self { squad_id, home }
    }
}
impl Kind for ComHealer {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [
            Part::Tough,
            Part::Heal,
            Part::Heal,
            Part::Move
        ];
        let basic_parts = [
            Part::Heal, Part::RangedAttack,
            Part::Heal, Part::RangedAttack,
            Part::Heal, Part::RangedAttack,
            Part::Heal, Part::RangedAttack,
            Part::Heal, Part::RangedAttack
        ];

        let mut body = basic_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
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

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        //todo change ticks to boost here when labs carry requests will be fixed
        if creep.ticks_to_live().is_some_and(|tick| tick > 1300) {
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
                Task::Boost(id, u32::try_from(parts_number).ok())
            })
            .or_else(|| self.squad_id
                .as_ref()
                .and_then(|sid| {
                    get_request(home, sid).and_then(|req| home.take_request(&req)).map(|mut req| {
                        req.join(Some(creep.name()), Some(sid));
                        home.add_request(req.clone());
                        (req, Role::CombatHealer(self.clone())).into()
                    })
                }))
            .unwrap_or_default()
    }
}

fn get_request(home: &Shelter, squad_id: &str) -> Option<Request> {
    home.requests()
        .find(|r| r.assigned_to(squad_id))
        .cloned()
}
