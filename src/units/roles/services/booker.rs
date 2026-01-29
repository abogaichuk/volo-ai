use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::prelude::*;
use screeps::{Part, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{Request, RequestKind};

#[derive(Clone, Serialize, Deserialize)]
pub struct Booker {
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Booker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl Booker {
    pub const fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Booker {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Move, Part::Claim];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 14) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.hits() > creep.hits_max() - creep.hits_max() / 2 {
            MovementProfile::PlainsOneToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        get_active_job(home, creep)
            .map(|req| req.kind.into())
            .or_else(|| {
                get_new_job(home).and_then(|req| home.take_request(&req)).map(|mut req| {
                    req.join(Some(creep.name()), None);
                    home.add_request(req.clone());
                    req.kind.into()
                })
            })
            .unwrap_or_default()
    }
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Book(_) if
            matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name()))
        })
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Book(_) if
            matches!(*r.status(), Status::Spawning))
        })
        .cloned()
}
