use serde::{Serialize, Deserialize};
use screeps::{Part, RoomName, objects::Creep, prelude::*};
use std::fmt;
use arrayvec::ArrayVec;
use crate::{
    movement::MovementProfile,
    rooms::{shelter::Shelter, state::requests::{Request, RequestKind, meta::Status}}
};
use super::{Kind, Task, can_scale, default_parts_priority};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Defender {
    pub(crate) home: Option<RoomName>
}

impl fmt::Debug for Defender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Defender {
    pub fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Defender {

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::RangedAttack, Part::Heal, Part::Move, Part::Move];

        let mut body = [Part::RangedAttack, Part::Move]
            .into_iter()
            .collect::<ArrayVec<[Part; 50]>>();

        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().cloned());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() < creep.hits_max() {
            MovementProfile::RoadsOneToTwo
        } else {
            MovementProfile::PlainsOneToOne
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
         home.get_available_boost(creep, self.boosts(creep))
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter()
                    .filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, Some(parts_number as u32))
            })
            .or_else(|| get_request(home)
                .and_then(|req| home.take_request(&req)
                .map(|mut req| {
                    req.join(Some(creep.name()), None);
                    home.add_request(req.clone());
                    req.kind.into()
                })))
            .unwrap_or_else(|| Task::Defend(creep.pos().room_name(), false))
    }
}

fn get_request(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| matches!(&r.kind, RequestKind::Defend(_) if
                matches!(*r.status(), Status::InProgress)))
        .or_else(|| home.requests()
            .find(|r| matches!(&r.kind, RequestKind::Defend(_) if
                matches!(*r.status(), Status::Created))))
        .cloned()
}