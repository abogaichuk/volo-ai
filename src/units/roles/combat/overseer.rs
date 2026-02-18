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
pub struct Overseer {
    pub(crate) workroom: Option<RoomName>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Overseer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {home}, ")?;
        }
        if let Some(workroom) = &self.workroom {
            write!(f, "workroom: {workroom}")?;
        }
        write!(f, "")
    }
}

impl Overseer {
    pub const fn new(workroom: Option<RoomName>, home: Option<RoomName>) -> Self {
        Self { workroom, home }
    }
}

impl Kind for Overseer {
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

    fn respawn_timeout(&self, creep: Option<&Creep>) -> Option<usize> {
        if self.workroom.is_some() {
            creep.map(|c| c.body().len() * 3 + 100).or(Some(0))
        } else {
            None
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        self.workroom
            .map(|rn| Task::Oversee(rn, None))
            .or_else(|| {
                get_request(home, creep).and_then(|req| {
                    home.take_request(&req).map(|mut req| {
                        req.join(Some(creep.name()), None);
                        home.add_request(req.clone());
                        req.kind.into()
                    })
                })
            })
            .unwrap_or_default()
    }
}

fn get_request(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Crash(_) if
            matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name()))
        })
        .or_else(|| {
            home.requests().find(|r| {
                matches!(&r.kind, RequestKind::Crash(_) if
                matches!(*r.status(), Status::Spawning))
            })
        })
        .cloned()
}
