use std::collections::HashMap;
use std::fmt;

use arrayvec::ArrayVec;
use screeps::{Creep, ObjectId, Part, Position, ResourceType, RoomName, StructureController};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::movement::walker::Walker;
use crate::rooms::shelter::Shelter;

#[derive(Clone, Serialize, Deserialize)]
pub struct RemoteUpgrader {
    pub(crate) home: Option<RoomName>,
    pub(crate) workplace: Option<Position>,
    ctrl: ObjectId<StructureController>,
    #[serde(default)]
    boost: bool,
}

impl fmt::Debug for RemoteUpgrader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home { write!(f, "home: {home}") } else { write!(f, "") }
    }
}

impl RemoteUpgrader {
    pub const fn new(
        home: Option<RoomName>,
        workplace: Option<Position>,
        ctrl: ObjectId<StructureController>,
        boost: bool,
    ) -> Self {
        Self { home, workplace, ctrl, boost }
    }
}

impl Kind for RemoteUpgrader {
    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.store().get_used_capacity(None) > 0 {
            MovementProfile::RoadsOneToTwo
        } else {
            MovementProfile::PlainsOneToOne
        }
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        if self.boost && creep.ticks_to_live().is_some_and(|tick| tick > 1450) {
            [(Part::Carry, [ResourceType::CatalyzedKeaniumAcid, ResourceType::KeaniumAcid])].into()
        } else {
            HashMap::new()
        }
    }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let (basic_parts, scale_parts): (SmallVec<[Part; 3]>, SmallVec<[Part; 7]>) = if self.boost {
            (
                smallvec![Part::Carry],
                smallvec![
                    Part::Work,
                    Part::Move,
                    Part::Work,
                    Part::Move,
                    Part::Work,
                    Part::Move,
                    Part::Carry
                ],
            )
        } else {
            (
                smallvec![Part::Work, Part::Carry, Part::Move],
                smallvec![Part::Work, Part::Carry, Part::Move],
            )
        };

        let mut body = basic_parts.into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, 50) {
            body.extend(scale_parts.iter().copied());
        }

        body.sort_by_key(|a| default_parts_priority(*a));
        body
    }

    fn respawn_timeout(&self, _: Option<&Creep>) -> Option<usize> {
        Some(750)
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        home.get_available_boost(creep, self.boosts(creep))
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter().filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, u32::try_from(parts_number).ok())
            })
            .or_else(|| {
                (creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0)
                    .then(|| Task::Upgrade(self.ctrl, None))
            })
            .or_else(|| {
                self.workplace.map(|workplace| {
                    if let Some(source) = home.find_source_near(workplace) {
                        Task::Harvest(workplace, source)
                    } else {
                        Task::MoveMe(workplace.room_name(), Walker::Exploring(false))
                    }
                })
            })
            .unwrap_or_default()
    }
}
