use serde::{Serialize, Deserialize};
use screeps::{Creep, HasId, HasPosition, Part, ResourceType, RoomName, game};
use std::{fmt, collections::HashMap, iter};
use arrayvec::ArrayVec;
use crate::{movement::MovementProfile,rooms::shelter::Shelter};
use super::{Kind, Task, can_scale, default_parts_priority};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Upgrader {
    pub(crate) home: Option<RoomName>
}

impl fmt::Debug for Upgrader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {}", home)
        } else {
            write!(f, "")
        }
    }
}

impl Upgrader {
    pub fn new(home: Option<RoomName>) -> Self {
        Self { home }
    }
}

impl Kind for Upgrader {

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Work, Part::Work, Part::Work, Part::Work, Part::Work, Part::Move];
        let mut body = iter::once(Part::Carry).collect::<ArrayVec<[Part; 50]>>();

        let parts_limit = limit_based_on_controller_level(self.home);
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, parts_limit) {
            body.extend(scale_parts.iter().cloned());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn get_movement_profile(&self, _: &Creep) -> MovementProfile {
        MovementProfile::RoadsOneToTwo
    }
    
    fn respawn_timeout(&self, creep: Option<&Creep>) -> Option<u32> {
        creep
            .map(|c| c.body().len() as u32 * 3)
            .or(Some(0))
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        if creep.ticks_to_live().is_some_and(|tick| tick > 1450) {
            [(Part::Work, [ResourceType::CatalyzedGhodiumAcid, ResourceType::GhodiumAcid])].into()
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
            .or_else(|| home.find_container_in_range(home.controller().pos(), 2)
                .map(|(id, pos)| {
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                        Task::Upgrade(home.controller().id(), Some(id))
                    } else {
                        Task::TakeFromStructure(pos, id, ResourceType::Energy, None)
                    }
                }))
            .unwrap_or_default()
    }
}

fn limit_based_on_controller_level(home: Option<RoomName>) -> usize {
    if home
        .and_then(|home| game::rooms().get(home))
        .and_then(|room| room.controller())
        .is_some_and(|ctrl| ctrl.level() == 8)
    {
        19
    } else {
        50
    }
}
