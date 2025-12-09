use std::collections::HashMap;
use std::fmt;

use arrayvec::ArrayVec;
use screeps::StructureType::{Extension, Factory, Lab, Link, Nuker, PowerSpawn, Spawn, Tower};
use screeps::{
    Creep, HasId, HasPosition, Part, ResourceType, RoomName, SharedCreepProperties,
    StructureObject, find,
};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{Request, RequestKind};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseKeeper {
    pub(crate) home: Option<RoomName>,
    #[serde(default)]
    boost: bool,
}

impl fmt::Debug for HouseKeeper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl HouseKeeper {
    pub const fn new(home: Option<RoomName>, boost: bool) -> Self {
        Self { home, boost }
    }
}

impl Kind for HouseKeeper {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Work, Part::Carry, Part::Move];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::RoadsOneToTwo
    }

    fn respawn_timeout(&self, creep: Option<&Creep>) -> Option<usize> {
        creep.map(|c| c.body().len() * 3).or(Some(0))
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        if creep.ticks_to_live().is_some_and(|tick| tick > 800) {
            [(Part::Work, [ResourceType::CatalyzedLemergiumAcid, ResourceType::LemergiumAcid])]
                .into()
        } else {
            HashMap::new()
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        home.get_available_boost(creep, self.boosts(creep))
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter().filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, u32::try_from(parts_number).ok())
            })
            .or_else(|| {
                (creep.store().get_used_capacity(Some(ResourceType::Energy)) == 0)
                    .then(|| find_energy(creep, home))
            })
            .or_else(|| {
                (home.invasion())
                    .then(|| {
                        home.lowest_perimetr_hits()
                            .map(|rampart| Task::Repair(rampart.raw_id().into(), rampart.pos(), 25))
                    })
                    .flatten()
            })
            .or_else(|| get_active_job(home, creep).map(|req| req.kind.into()))
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

fn find_energy(creep: &Creep, home: &Shelter) -> Task {
    (creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0)
        .then(|| {
            get_closest_energy_container(creep).map(|o| {
                Task::TakeFromStructure(
                    o.pos(),
                    o.as_structure().raw_id(),
                    ResourceType::Energy,
                    None,
                )
            })
        })
        .flatten()
        .unwrap_or_else(|| {
            home.storage()
                .filter(|storage| {
                    storage.store().get_used_capacity(Some(ResourceType::Energy)) > 10000
                })
                .map(|storage| {
                    Task::TakeFromStructure(
                        storage.pos(),
                        storage.raw_id(),
                        ResourceType::Energy,
                        None,
                    )
                })
                .unwrap_or_default()
        })
}

fn get_closest_energy_container(creep: &Creep) -> Option<StructureObject> {
    let room = creep.room().expect("couldn't resolve a room");
    let mut energy_containers: Vec<StructureObject> = room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter(|s| {
            let s_type = s.as_structure().structure_type();
            s_type != Tower
                && s_type != Link
                && s_type != Extension
                && s_type != Spawn
                && s_type != Lab
                && s_type != PowerSpawn
                && s_type != Nuker
                && s_type != Factory
                && has_energy(s)
        })
        .collect();

    energy_containers
        .sort_by_key(|container: &StructureObject| container.pos().get_range_to(creep.pos()));

    energy_containers.reverse();
    energy_containers.pop()
}

fn has_energy(structure: &StructureObject) -> bool {
    structure
        .as_has_store()
        .is_some_and(|storeable| storeable.store().get_used_capacity(Some(ResourceType::Energy)) > 500)
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| matches!(&r.kind, RequestKind::Repair(_) | RequestKind::Build(_)
            if matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name())))
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    get_earlier_build(home).or_else(|| get_most_out_of_order(home)).cloned()
}

fn get_earlier_build<'a>(home: &'a Shelter) -> Option<&'a Request> {
    home.requests()
        .filter(|r| {
            matches!(&r.kind, RequestKind::Build(_) if
            matches!(*r.status(), Status::Created))
        })
        .min_by_key(|r| r.created_at())
}

fn get_most_out_of_order<'a>(home: &'a Shelter) -> Option<&'a Request> {
    home.requests()
        .filter(|r| {
            matches!(&r.kind, RequestKind::Repair(_) if
            matches!(*r.status(), Status::Created))
        })
        .min_by(|first, second| match &first.kind {
            RequestKind::Repair(first) => match &second.kind {
                RequestKind::Repair(second) => first.hits.cmp(&second.hits),
                _ => std::cmp::Ordering::Greater,
            },
            _ => std::cmp::Ordering::Greater,
        })
}
