use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::prelude::*;
use screeps::{Part, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{Request, RequestKind};
use crate::units::Role;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DepositMiner {
    pub(crate) squad_id: Option<String>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for DepositMiner {
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

impl DepositMiner {
    pub const fn new(squad_id: Option<String>, home: Option<RoomName>) -> Self {
        Self { squad_id, home }
    }
}

impl Kind for DepositMiner {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [
            Part::Work,
            Part::Work,
            Part::Work,
            Part::Work,
            Part::Carry,
            Part::Move,
            Part::Move,
            Part::Move,
            Part::Move,
        ];

        let mut body = [Part::Work, Part::Work, Part::Carry, Part::Move, Part::Move]
            .into_iter()
            .collect::<ArrayVec<[Part; 50]>>();

        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body
    }

    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::PlainsOneToOne
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        self.squad_id
            .as_ref()
            .and_then(|sid| {
                get_request(home, sid).and_then(|req| home.take_request(&req)).map(|mut req| {
                    req.join(Some(creep.name()), Some(sid));
                    home.add_request(req.clone());

                    (req, Role::DepositMiner(self.clone())).into()
                })
            })
            .unwrap_or_default()
    }
}

fn get_request(home: &Shelter, squad_id: &str) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Deposit(_) if
            matches!(*r.status(), Status::InProgress | Status::Carry) && r.assigned_to(squad_id))
        })
        .cloned()
}
