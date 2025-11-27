use log::info;
use serde::{Serialize, Deserialize};
use screeps::{objects::Creep, prelude::*, Part, RoomName};
use std::fmt;
use arrayvec::ArrayVec;
use crate::{
    movement::MovementProfile,
    rooms::{shelter::Shelter, state::requests::{Request, RequestKind, meta::Status}}
};
use super::{Kind, Task};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conqueror {
    pub(crate) home: Option<RoomName>
}

impl fmt::Debug for Conqueror {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Conqueror {
    pub fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Conqueror {

    fn body(&self, _: u32) -> ArrayVec<[Part; 50]> {
        [Part::Move, Part::Move, Part::Move, Part::Move, Part::Move, Part::Claim].into_iter().collect()
    }

    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::SwampFiveToOne
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        get_active_job(home, creep)
            .map(|req| req.kind.into())
            .or_else(|| get_new_job(home)
                .and_then(|req| home.take_request(&req))
                .map(|mut req| {
                    req.join(Some(creep.name()), None);
                    home.add_request(req.clone());
                    req.kind.into()
                }))
            .inspect(|task| info!("{} found task: {:?}", creep.name(), task))
            .unwrap_or_default()
    }
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| matches!(&r.kind, RequestKind::Claim(_) if
            matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name())))
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| matches!(&r.kind, RequestKind::Claim(_) if
            matches!(*r.status(), Status::Spawning)))
        .cloned()
}