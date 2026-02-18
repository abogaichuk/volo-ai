use std::collections::HashMap;
use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{Part, ResourceType, RoomName};
use serde::{Deserialize, Serialize};

use super::Kind;
use crate::movement::MovementProfile;
use crate::units::roles::can_scale;

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

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [
            Part::Tough,
            Part::Work,
            Part::Work,
            Part::Work,
            Part::Move
        ];
        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body
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
                    Part::Work,
                    [ResourceType::CatalyzedZynthiumAcid, ResourceType::ZynthiumAcid],
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
}

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
