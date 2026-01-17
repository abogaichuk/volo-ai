use std::fmt;

use arrayvec::ArrayVec;
use log::warn;
use screeps::constants::ResourceType;
use screeps::objects::Creep;
use screeps::prelude::*;
use screeps::{Part, RoomName};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{Request, RequestKind};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hauler {
    pub(crate) home: Option<RoomName>,
    #[serde(default)]
    periodic: bool,
}

impl fmt::Debug for Hauler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl Hauler {
    pub const fn new(home: Option<RoomName>, periodic: bool) -> Self {
        Self { home, periodic }
    }
}

impl Kind for Hauler {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Carry, Part::Carry, Part::Move];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
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
                    .then(|| {
                        let resource = creep
                            .store()
                            .store_types()
                            .into_iter()
                            .next()
                            .expect("expect resource in a creep!");

                        if let Some(structure) = home.closest_empty_structure(creep)
                            && can_fill(structure.free_capacity(), creep)
                        {
                            return Task::FillStructure(structure);
                        }

                        if let Some(storage) = home
                            .storage()
                            .filter(|storage| storage.store().get_free_capacity(None) > 5000)
                        {
                            Task::DeliverToStructure(
                                storage.pos(),
                                storage.raw_id(),
                                resource,
                                None,
                            )
                        } else {
                            warn!(
                                "{} {} there is no place to store! drop?",
                                home.name(),
                                creep.name()
                            );
                            let _ = creep.drop(resource, None);
                            Task::Idle(1)
                        }
                    })
                    .or_else(|| {
                        get_new_job(home).and_then(|req| home.take_request(&req)).map(|mut req| {
                            // req.begin(creep.name());
                            req.join(Some(creep.name()), None);
                            home.add_request(req.clone());
                            req.kind.into()
                        })
                    })
            })
            .or_else(|| take_energy(home, creep))
            .unwrap_or_default()
    }
}

fn can_fill(str_free_capacity: i32, creep: &Creep) -> bool {
    let energy_in_store = creep.store().get_used_capacity(Some(ResourceType::Energy));
    u32::try_from(str_free_capacity).ok().is_some_and(|free_capacity| {
        free_capacity <= energy_in_store || energy_in_store == creep.store().get_capacity(None)
    })
}

fn take_energy(home: &Shelter, creep: &Creep) -> Option<Task> {
    (home.closest_empty_structure(creep).is_some())
        .then(|| {
            home.storage()
                .filter(|storage| {
                    storage.store().get_used_capacity(Some(ResourceType::Energy)) > 2000
                })
                .map(|storage| {
                    Task::TakeFromStructure(
                        storage.pos(),
                        storage.raw_id(),
                        ResourceType::Energy,
                        None,
                    )
                })
                .or_else(|| {
                    home.factory()
                        .filter(|factory| {
                            factory.store().get_used_capacity(Some(ResourceType::Energy)) > 1000
                        })
                        .map(|factory| {
                            Task::TakeFromStructure(
                                factory.pos(),
                                factory.raw_id(),
                                ResourceType::Energy,
                                None,
                            )
                        })
                })
        })
        .flatten()
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Withdraw(_) | RequestKind::Pickup(_)
            if matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name()))
        })
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| {
            matches!(&r.kind, RequestKind::Withdraw(_) | RequestKind::Pickup(_)
            if matches!(*r.status(), Status::Created))
        })
        .cloned()
}
