use std::fmt;

use arrayvec::ArrayVec;
use screeps::objects::Creep;
use screeps::{EffectType, Part, Position, PowerType, RoomName, find, game};
use screeps::{SOURCE_ENERGY_KEEPER_CAPACITY, prelude::*};
use serde::{Deserialize, Serialize};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Miner {
    pub(crate) workplace: Option<Position>,
    pub(crate) home: Option<RoomName>,
}

impl fmt::Debug for Miner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(home) = self.home {
            write!(f, "home: {home}, ")?;
        }
        if let Some(workplace) = &self.workplace {
            write!(f, "workplace: {workplace}")?;
        }
        write!(f, "")
    }
}

impl Miner {
    pub const fn new(workplace: Option<Position>, home: Option<RoomName>) -> Self {
        Self { workplace, home }
    }
}

impl Kind for Miner {
    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]> {
        let scale_parts = [Part::Work, Part::Work, Part::Move];

        let parts_limit = limit_based_on_source_effects(self.workplace);
        let mut body = [Part::Carry].into_iter().collect::<ArrayVec<[Part; 50]>>();
        while can_scale(body.clone(), scale_parts.to_vec(), room_energy, parts_limit) {
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

    fn get_task(&self, _: &Creep, home: &mut Shelter) -> Task {
        self.workplace
            .and_then(|workplace| {
                home.all_sources()
                    .find(|s| workplace.is_near_to(s.pos()))
                    .map(|s| Task::HarvestEnergyForever(workplace, s.id()))
            })
            .unwrap_or_default()
    }
}

fn limit_based_on_source_effects(workplace: Option<Position>) -> usize {
    workplace
        .and_then(|workplace| {
            game::rooms().get(workplace.room_name()).and_then(|room| {
                room.find(find::SOURCES, None)
                    .iter()
                    .find(|source| source.pos().is_near_to(workplace))
                    .and_then(|source| {
                        source
                            .effects()
                            .into_iter()
                            .find(|effect| match effect.effect() {
                                EffectType::PowerEffect(p) => matches!(p, PowerType::RegenSource),
                                EffectType::NaturalEffect(_) => false,
                            })
                            .and_then(|effect| match effect.level() {
                                Some(1) => Some(13),
                                Some(2) => Some(16),
                                Some(3) => Some(19),
                                Some(4) => Some(22),
                                Some(5) => Some(25),
                                _ => Some(10),
                            })
                            .or_else(|| {
                                (source.energy_capacity() == SOURCE_ENERGY_KEEPER_CAPACITY)
                                    .then(|| 19)
                            })
                    })
            })
        })
        .unwrap_or(10)
}

// fn limit_based_on_source_effects(workplace: Option<Position>) -> usize {
//     workplace
//         .and_then(|workplace| {
//             game::rooms().get(workplace.room_name()).and_then(|room| {
//                 room.find(find::SOURCES, None)
//                     .iter()
//                     .find(|source| source.pos().is_near_to(workplace))
//                     .map(screeps::RoomObjectProperties::effects)
//                     .and_then(|effects| {
//                         effects.into_iter().find(|effect| match effect.effect() {
//                             EffectType::PowerEffect(p) => matches!(p, PowerType::RegenSource),
//                             EffectType::NaturalEffect(_) => false,
//                         })
//                     })
//                     .and_then(|effect| effect.level())
//                     .map(|level| match level {
//                         1 => 13,
//                         2 => 16,
//                         3 => 19,
//                         4 => 22,
//                         5 => 25,
//                         _ => 10,
//                     })
//             })
//         })
//         .unwrap_or(10)
// }
