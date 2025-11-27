use serde::{Serialize, Deserialize};
use screeps::{Creep, HasId, Part, ResourceType, RoomName, SharedCreepProperties};
use std::{collections::HashMap, fmt};
use arrayvec::ArrayVec;
use crate::{
    movement::MovementProfile,
    rooms::{shelter::Shelter, state::requests::{Request, RequestKind, meta::Status}}
};
use super::{Kind, Task, can_scale};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dismantler {
    pub(crate) home: Option<RoomName>
}

impl fmt::Debug for Dismantler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Dismantler {
    pub fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Dismantler {

    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::Cargo
    }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Work];

        let mut body = scale_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(
            body.clone(),
            scale_parts.to_vec(),
            room_energy,
            50)
        {
            body.extend(scale_parts.iter().cloned());
        }

        body
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        if creep.ticks_to_live().is_some_and(|tick| tick > 1400) {
            [(Part::Work, [ResourceType::CatalyzedZynthiumAcid, ResourceType::ZynthiumAcid])].into()
        } else {
            HashMap::new()
        }
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        home.get_available_boost(creep, self.boosts(creep))
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter()
                    .filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, Some(parts_number as u32))
            })
            .or_else(|| get_active_job(home, creep)
                .map(|req| req.kind.into())
                .or_else(|| get_new_job(home)
                    .and_then(|req| home.take_request(&req))
                    .map(|mut req| {
                        req.join(Some(creep.name()), None);
                        home.add_request(req.clone());
                        req.kind.into()
                    })))
            .unwrap_or_default()
    }
}

fn get_active_job(home: &Shelter, creep: &Creep) -> Option<Request> {
    home.requests()
        .find(|r| matches!(&r.kind, RequestKind::Dismantle(_) if
            matches!(*r.status(), Status::InProgress) && r.assigned_to(&creep.name())))
        .cloned()
}

fn get_new_job(home: &Shelter) -> Option<Request> {
    home.requests()
        .find(|r| matches!(&r.kind, RequestKind::Dismantle(_) if
            matches!(*r.status(), Status::Created | Status::Spawning)))
        .cloned()
}