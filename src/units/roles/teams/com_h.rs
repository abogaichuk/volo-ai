use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{Part, RoomName};
use serde::{Deserialize, Serialize};

use super::Kind;
use crate::movement::MovementProfile;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComHealer {
    pub(crate) squad_id: Option<String>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for ComHealer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}, ", home)?;
        }
        if let Some(squad_id) = &self.squad_id {
            write!(f, "squad_id: {}", squad_id)?;
        }
        write!(f, "")
    }
}

impl ComHealer {
    pub fn new(squad_id: Option<String>, home: Option<RoomName>) -> Self {
        Self { squad_id, home }
    }
}
impl Kind for ComHealer {
    // fn body_config(&self) -> BodyConfig {
    //     BodyConfig::new(
    //         vec![Part::Heal, Part::Heal, Part::Heal, Part::Heal, Part::Tough,
    // Part::Tough, Part::Tough, Part::Move, Part::Move, Part::RangedAttack],
    //         vec![Part::Heal, Part::Heal, Part::Heal, Part::Heal, Part::Tough,
    // Part::Tough, Part::Tough, Part::Move, Part::Move, Part::RangedAttack],
    //         50)
    // }
    fn body(&self, _: u32) -> ArrayVec<[Part; 50]> {
        [Part::Move].into_iter().collect()
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() > creep.hits_max() - creep.hits_max() / 5 {
            MovementProfile::PlainsOneToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }
}

// fn all_boosts() -> HashMap<Part, [ResourceType; 2]> {
//     let mut m = HashMap::new();
//     m.insert(Part::Move, [ResourceType::CatalyzedZynthiumAlkalide,
// ResourceType::ZynthiumAlkalide]);     m.insert(Part::RangedAttack,
// [ResourceType::CatalyzedKeaniumAlkalide, ResourceType::KeaniumAlkalide]);
//     m.insert(Part::Heal, [ResourceType::CatalyzedLemergiumAlkalide,
// ResourceType::LemergiumAlkalide]);     m.insert(Part::Tough,
// [ResourceType::CatalyzedGhodiumAlkalide, ResourceType::GhodiumAlkalide]);
//     m
// }

// fn is_wounded(creep: &Creep) -> bool {
//     // creep.hits() <= creep.hits_max() - creep.hits_max() / 10 // <= than
// 90% == 5 parts     creep.hits() <= creep.hits_max() - creep.hits_max() / 20
// // <= than 93.33% == 3.3% parts }

// fn get_request(requests: &mut HashSet<RoomRequest>, squad_id: &String) ->
// Option<RoomRequest> {     requests.iter()
//         .find(|request| {
//             match request {
//                 RoomRequest::DESTROY(destroy_request) =>
//                     destroy_request.status == RequestStatus::InProgress &&
// destroy_request.squads.iter()                         .any(|squad| squad.id
// == *squad_id),                 _ => false
//             }
//         })
//         .cloned()
//         .and_then(|request| requests.take(&request))
// }
