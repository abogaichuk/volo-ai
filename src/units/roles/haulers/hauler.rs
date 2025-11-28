use log::*;
use serde::{Serialize, Deserialize};
use screeps::{
    constants::ResourceType, objects::Creep, prelude::*, RoomName, Part
};
use std::fmt;
use crate::{
    movement::MovementProfile,
    rooms::{shelter::Shelter, state::requests::{Request, RequestKind, meta::Status}},
    commons::{find_empty_structures, extension_capacity}
};
use arrayvec::ArrayVec;
use super::{Kind, Task, can_scale, default_parts_priority};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hauler {
    pub(crate) home: Option<RoomName>
}

impl fmt::Debug for Hauler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Hauler {
    pub fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Hauler {

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Carry, Part::Carry, Part::Move];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().cloned());
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

    fn respawn_timeout(&self, creep: Option<&Creep>) -> Option<u32> {
        creep
            .map(|c| c.body().len() as u32 * 3)
            .or(Some(0))
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        get_active_job(home, creep)
            .map(|req| req.kind.into())
            .or_else(|| (creep.store().get_used_capacity(None) > 0)
                .then(|| {
                    let resource = creep.store().store_types().into_iter().next()
                        .expect("expect resource in a creep!");

                    if let Some(structure) = home.closest_empty_structure(creep) {
                        if resource == ResourceType::Energy && structure.free_capacity() < creep.store().get_used_capacity(Some(resource)) as i32 {
                            
                            return Task::FillStructure(Box::new(structure));
                        } 
                    }

                    if let Some(storage) = home.storage()
                        .filter(|storage| storage.store().get_free_capacity(None) > 5000)
                    {
                        Task::DeliverToStructure(storage.pos(), storage.raw_id(), resource, None)
                    } else {
                        warn!("{} {} there is no place to store! drop?", home.name(), creep.name());
                        let _ = creep.drop(resource, None);
                        Task::Idle(1)
                    }
                    // if creep.store().get_used_capacity(Some(ResourceType::Energy)) >= extension_capacity(home.room()) &&
                    //     find_empty_structures(home.room()).count() > 0
                    // {
                    //     Task::FillStructures(home.name())
                    // } else if let Some(storage) = home.storage()
                    //     .filter(|storage| storage.store().get_free_capacity(None) > 10000)
                    // {
                    //     Task::DeliverToStructure(storage.pos(), storage.raw_id(), resource, None)
                    // } else {
                        // warn!("{} {} there is no place to store! drop?", home.name(), creep.name());
                        // let _ = creep.drop(resource, None);
                        // Task::Idle(1)
                    // }
                })
                .or_else(|| get_new_job(home)
                    .and_then(|req| home.take_request(&req))
                    .map(|mut req| {
                        // req.begin(creep.name());
                        req.join(Some(creep.name()), None);
                        home.add_request(req.clone());
                        req.kind.into()
                    }))
                )
                .or_else(|| try_fill(home))
            .unwrap_or_default()
    }
}

fn try_fill(home: &Shelter) -> Option<Task> {
    (find_empty_structures(home.room()).count() > 0)
        .then(|| home.storage()
            .filter(|storage| storage.store().get_used_capacity(Some(ResourceType::Energy)) > 2000)
            .map(|storage| Task::TakeFromStructure(storage.pos(), storage.raw_id(), ResourceType::Energy, None))
            .or_else(|| home.factory()
                .filter(|factory| factory.store().get_used_capacity(Some(ResourceType::Energy)) > 1000)
                .map(|factory| Task::TakeFromStructure(factory.pos(), factory.raw_id(), ResourceType::Energy, None))))
        .flatten()
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| {
            match &r.kind {
                RequestKind::Withdraw(_) if matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name()) => true,
                RequestKind::Pickup(_) if matches!(*r.status(), Status::InProgress)&& r.assigned_to(&creep.name()) => true,
                _ => false
            }
        })
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| {
            match &r.kind {
                RequestKind::Withdraw(_) if matches!(*r.status(), Status::Created) => true,
                RequestKind::Pickup(_) if matches!(*r.status(), Status::Created) => true,
                _ => false
            }
        })
        .cloned()
}