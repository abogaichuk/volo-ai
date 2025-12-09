use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{Part, RoomName};
use serde::{Deserialize, Serialize};

use super::Kind;
use crate::movement::MovementProfile;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComDismantler {
    pub(crate) squad_id: Option<String>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for ComDismantler {
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

impl ComDismantler {
    pub const fn new(squad_id: Option<String>, home: Option<RoomName>) -> Self {
        Self { squad_id, home }
    }
}

impl Kind for ComDismantler {
    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() > creep.hits_max() - creep.hits_max() / 5 {
            MovementProfile::PlainsOneToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }

    fn body(&self, _: u32) -> ArrayVec<[Part; 50]> {
        [Part::Move].into_iter().collect()
    }
}

// fn all_boosts() -> HashMap<Part, [ResourceType; 2]> {
//     let mut m = HashMap::new();
//     m.insert(Part::Move, [ResourceType::CatalyzedZynthiumAlkalide,
// ResourceType::ZynthiumAlkalide]);     m.insert(Part::Work,
// [ResourceType::CatalyzedZynthiumAcid, ResourceType::ZynthiumAcid]);
//     m.insert(Part::Tough, [ResourceType::CatalyzedGhodiumAlkalide,
// ResourceType::GhodiumAlkalide]);     m
// }

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
