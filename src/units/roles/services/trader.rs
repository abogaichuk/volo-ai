use std::fmt;

use arrayvec::ArrayVec;
use log::{debug, warn};
use screeps::{Creep, HasId, HasPosition, Part, ResourceType, RoomName, SharedCreepProperties};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{Request, RequestKind};
use crate::utils::constants::MIN_ENERGY_CAPACITY;

#[derive(Clone, Serialize, Deserialize)]
pub struct Trader {
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Trader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl Trader {
    pub const fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Trader {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Carry, Part::Carry, Part::Move];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 30) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.store().get_used_capacity(None) == 0 {
            MovementProfile::SwampFiveToOne
        } else {
            MovementProfile::RoadsOneToTwo
        }
    }

    fn respawn_timeout(&self, creep: Option<&Creep>) -> Option<usize> {
        creep.map(|c| c.body().len() * 3).or(Some(0))
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        get_active_job(home, creep)
            .map(|req| req.kind.into())
            .or_else(|| {
                (creep.store().get_used_capacity(None) > 0)
                    //if no active request found, but something is in store - get rid of it
                    .then(|| {
                        let resource = creep
                            .store()
                            .store_types()
                            .into_iter()
                            .next()
                            .expect("expect something in the creep store");
                        if let Some(sender) = home.empty_sender()
                            && resource == ResourceType::Energy
                        {
                            //if resource energy and sender link is empty -> fill it
                            Task::DeliverToStructure(
                                sender.pos(),
                                sender.raw_id(),
                                ResourceType::Energy,
                                None,
                            )
                        } else if let Some(bank) = home
                            .storage()
                            .filter(|storage| storage.store().get_free_capacity(None) > 10_000)
                        {
                            Task::DeliverToStructure(bank.pos(), bank.raw_id(), resource, None)
                        } else {
                            warn!("{} no free storage space here!", home.name());
                            // let _ = creep.drop(resource, None);
                            Task::Idle(1)
                        }
                    })
            })
            .or_else(|| {
                home.full_receiver()
                    //empty creep here and receiver link is full -> take all energy from receiver
                    .map(|link| {
                        Task::TakeFromStructure(
                            link.pos(),
                            link.raw_id(),
                            ResourceType::Energy,
                            None,
                        )
                    })
            })
            .or_else(|| {
                home.empty_sender()
                    .is_some()
                    .then(|| {
                        home.storage()
                            .filter(|s| {
                                s.store().get_used_capacity(Some(ResourceType::Energy))
                                    > MIN_ENERGY_CAPACITY
                            })
                            //empty creep + empty sender link + storage has enough energy
                            .map(|storage| {
                                Task::TakeFromStructure(
                                    storage.pos(),
                                    storage.raw_id(),
                                    ResourceType::Energy,
                                    None,
                                )
                            })
                    })
                    .flatten()
            })
            // get new job if more then 10 ticks
            .or_else(|| {
                (creep.ticks_to_live().is_some_and(|ticks| ticks > 10))
                    .then(|| {
                        get_new_job(home).and_then(|req| home.take_request(&req)).map(|mut req| {
                            req.join(Some(creep.name()), None);
                            home.add_request(req.clone());
                            debug!("{}: {} got_new_job: {}", home.name(), creep.name(), req);
                            req.kind.into()
                        })
                    })
                    .flatten()
            })
            .unwrap_or_default()
    }
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Carry(_) if
            matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name()))
        })
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Carry(_) if
            matches!(*r.status(), Status::Created))
        })
        .cloned()
}
