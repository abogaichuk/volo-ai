use std::fmt;

use arrayvec::ArrayVec;
use log::warn;
use screeps::{Creep, HasId, HasPosition, Part, RoomName, SharedCreepProperties};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{Request, RequestKind};

#[derive(Clone, Serialize, Deserialize)]
pub struct Carrier {
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Carrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl Carrier {
    pub const fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Carrier {
    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        //todo bug here fix Long Range withdraw to carry back it to storage, long range
        // carry!
        if creep.store().get_used_capacity(None) > 0 {
            let resource = creep
                .store()
                .store_types()
                .into_iter()
                .next()
                .expect("expect resource in a creep!");

            if let Some(storage) = home.storage() {
                Task::DeliverToStructure(storage.pos(), storage.raw_id(), resource, None)
            } else {
                warn!("{} {} there is no place to store! drop?", home.name(), creep.name());
                let _ = creep.drop(resource, None);
                Task::Idle(1)
            }
        } else if creep.ticks_to_live().is_some_and(|ticks| ticks > 350)
            && let Some(mut request) =
                get_request(home, creep).and_then(|req| home.take_request(&req))
        {
            // request.begin(creep.name());
            request.join(Some(creep.name()), None);
            home.add_request(request.clone());
            request.kind.into()
        } else {
            let _ = creep.suicide();
            Task::Idle(1)
        }
    }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Carry, Part::Move];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.store().get_used_capacity(None) == 0 {
            MovementProfile::SwampFiveToOne
        } else {
            MovementProfile::PlainsOneToOne
        }
    }
}

fn get_request(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::LongRangeWithdraw(_) if
            matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name()))
        })
        .or_else(|| {
            home.requests().find(|r| {
                matches!(&r.kind, RequestKind::LongRangeWithdraw(_) if
                matches!(*r.status(), Status::Spawning))
            })
        })
        .cloned()
}
