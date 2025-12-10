#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use arrayvec::ArrayVec;
use log::{warn, debug};
use screeps::constants::look::STRUCTURES;
use screeps::objects::Creep;
use screeps::prelude::*;
use screeps::{
    ConstructionSite, Part, Position, ResourceType, Room, RoomName, StructureObject,
    StructureRampart, find, game,
};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use super::{Kind, Task, can_scale, default_parts_priority};
use crate::commons::{find_hostiles, find_source_near, remoted_from_edge};
use crate::movement::MovementProfile;
use crate::rooms::shelter::Shelter;
use crate::utils::constants::{HANDYMAN_ENERGY_PICKUP_THRESHOLD, NO_TASK_IDLE_TICKS};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandyMan {
    workplace: Option<Position>,
    pub(crate) home: Option<RoomName>,
    #[serde(default)]
    boost: bool,
}

impl fmt::Debug for HandyMan {
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

impl HandyMan {
    pub const fn new(workplace: Option<Position>, home: Option<RoomName>, boost: bool) -> Self {
        Self { workplace, home, boost }
    }
}

impl Kind for HandyMan {
    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile {
        if creep.store().get_used_capacity(None) > 0 {
            MovementProfile::RoadsOneToTwo
        } else {
            MovementProfile::PlainsOneToOne
        }
    }

    fn boosts(&self, creep: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        if creep.ticks_to_live().is_some_and(|tick| tick > 1450) {
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
        Some(800)
    }

    fn get_task(&self, creep: &Creep, home: &mut Shelter) -> Task {
        home.get_available_boost(creep, self.boosts(creep))
            .map(|(id, body_part)| {
                let parts_number = creep.body().iter().filter(|bp| bp.part() == body_part).count();
                Task::Boost(id, u32::try_from(parts_number).ok())
            })
            .or_else(|| {
                self.workplace.map(|workplace| {
                    if creep.pos().room_name() == workplace.room_name()
                        && let Some(room) = game::rooms().get(workplace.room_name())
                    {
                        let hostiles: Vec<Creep> =
                            find_hostiles(&room, vec![Part::RangedAttack, Part::Attack]).collect();
                        if hostiles.iter().any(|hostile| remoted_from_edge(hostile.pos(), 4))
                            && room
                                .controller()
                                .is_some_and(|ctrl| ctrl.safe_mode().is_none_or(|ticks| ticks < 10))
                        {
                            if let Some(rampart) = get_rampart(workplace) {
                                if !creep.pos().is_equal_to(workplace) {
                                    if let Some(source) = find_source_near(workplace, &room) {
                                        Task::Harvest(workplace, source.id())
                                    } else {
                                        warn!("{} no source found!", creep.name());
                                        Task::Idle(1)
                                    }
                                } else if creep
                                    .store()
                                    .get_used_capacity(Some(ResourceType::Energy))
                                    > 0
                                {
                                    find_invasion_task(creep, rampart, &room, &hostiles)
                                } else {
                                    find_energy_near(creep, rampart, &room, &hostiles)
                                }
                            } else {
                                Task::Flee(9)
                            }
                        } else if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                            find_startup_task(creep, &room)
                        } else {
                            find_energy_or_source(&room, workplace)
                        }
                    } else {
                        Task::MoveMe(
                            workplace.room_name(),
                            crate::movement::walker::Walker::Exploring(false),
                        )
                    }
                })
            })
            .unwrap_or_default()
    }
}

fn get_rampart(workplace: Position) -> Option<StructureRampart> {
    workplace.look_for(STRUCTURES).into_iter().find_map(|structures| {
        structures.into_iter().find_map(|s| match s {
            StructureObject::StructureRampart(r) => Some(r),
            _ => None,
        })
    })
}

fn find_invasion_task(creep: &Creep, _: StructureRampart, room: &Room, _: &[Creep]) -> Task {
    fiil_near(creep, room)
        .or_else(|| build_near(creep, room))
        .or_else(|| repair_near(creep, room))
        .unwrap_or(Task::Idle(1))
}

fn build_near(creep: &Creep, room: &Room) -> Option<Task> {
    room.find(find::MY_CONSTRUCTION_SITES, None).iter().find_map(|c| {
        if c.pos().get_range_to(creep.pos()) < 4 {
            Some(Task::Build(c.try_id(), c.pos()))
        } else {
            None
        }
    })
}

fn repair_near(creep: &Creep, room: &Room) -> Option<Task> {
    room.find(find::STRUCTURES, None)
        .iter()
        .filter_map(|s| {
            if s.pos().get_range_to(creep.pos()) < 4 {
                match s {
                    StructureObject::StructureRampart(r) if r.my() && r.hits() < r.hits_max() => {
                        Some((Task::Repair(r.id().into_type(), r.pos(), 15), r.hits()))
                    }
                    StructureObject::StructureContainer(c)
                        if c.hits() < (c.hits_max() as f32 * 0.75) as u32 =>
                    {
                        Some((Task::Repair(c.id().into_type(), c.pos(), 5), c.hits()))
                    }
                    StructureObject::StructureRoad(r)
                        if r.hits() < (r.hits_max() as f32 * 0.5) as u32 =>
                    {
                        Some((Task::Repair(r.id().into_type(), r.pos(), 2), r.hits()))
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
        .min_by_key(|(_, hits)| *hits)
        .map(|(request, _)| request)
    // .sorted_by_key(|(_, hits)| *hits)
    // .map(|(request, _)| request)
    // .next()
}

#[allow(clippy::cast_possible_wrap)]
fn fiil_near(creep: &Creep, room: &Room) -> Option<Task> {
    room.find(find::STRUCTURES, None).iter().find_map(|s| match s {
        StructureObject::StructureTower(t)
            if t.pos().is_near_to(creep.pos())
                && t.store().get_free_capacity(Some(ResourceType::Energy))
                    > (t.store().get_capacity(Some(ResourceType::Energy)) / 2) as i32 =>
        {
            Some(Task::DeliverToStructure(
                s.pos(),
                s.as_structure().raw_id(),
                ResourceType::Energy,
                None,
            ))
        }
        _ => None,
    })
}

fn find_startup_task(creep: &Creep, room: &Room) -> Task {
    // look for supply tasks a spawn or extension
    debug!("startup handy is looking for job");
    for structure in room.find(find::STRUCTURES, None) {
        let (store, structure) = match structure {
            // for the three object types that are important to fill, snag their store then cast
            // them right back to StructureObject
            StructureObject::StructureSpawn(ref o) => (o.store(), structure),
            StructureObject::StructureExtension(ref o) => (o.store(), structure),
            StructureObject::StructureTower(ref o) => (o.store(), structure),
            _ => {
                // no need to deliver to any other structures with these little ones
                continue;
            }
        };

        if store.get_free_capacity(Some(ResourceType::Energy))
            > (store.get_capacity(Some(ResourceType::Energy)) / 2) as i32
        {
            return Task::DeliverToStructure(
                structure.pos(),
                structure.as_structure().raw_id(),
                ResourceType::Energy,
                None,
            );
        }
    }

    // look for repair tasks
    // note that we're using STRUCTURES instead of MY_STRUCTURES
    // so we can catch roads, containers, and walls
    for structure in room.find(find::STRUCTURES, None) {
        match structure {
            // StructureObject::StructureRampart(r) if r.my() && r.hits() < (r.hits_max() as f32 *
            // 0.5) as u32 => { StructureObject::StructureRampart(r) if r.my() &&
            // r.hits() < 10000000 => {
            StructureObject::StructureRampart(r) if r.my() && r.hits() < 15000 => {
                return Task::Repair(r.id().into_type(), r.pos(), 10);
            }
            // StructureObject::StructureRoad(r) if r.hits() < (r.hits_max() as f32 * 0.5) as u32 =>
            // {     return Task::Repair(r.id().into_type(), r.pos(), 2)
            // },
            StructureObject::StructureContainer(c)
                if c.hits() < ((c.hits_max() as f32 * 0.5) as u32) =>
            {
                return Task::Repair(c.id().into_type(), c.pos(), 5);
            }
            // StructureObject::StructureWall(w) if w.hits() < 100000 => {
            //     return Task::Repair(w.id().into_type(), w.pos(), 10)
            // }
            _ => {}
        }
    }

    // look for construction tasks next
    if let Some(construction_site) = room.find(find::MY_CONSTRUCTION_SITES, None).into_iter().fold(
        None,
        |acc: std::option::Option<ConstructionSite>, another| {
            if let Some(cs) = acc {
                match another
                    .pos()
                    .get_range_to(creep.pos())
                    .cmp(&cs.pos().get_range_to(creep.pos()))
                {
                    Ordering::Less => Some(another),
                    _ => Some(cs),
                }
            } else {
                Some(another)
            }
        },
    ) {
        Task::Build(construction_site.try_id(), construction_site.pos())
    } else if let Some(controller) = room.controller() {
        Task::Upgrade(controller.id(), None)
    } else {
        Task::Idle(NO_TASK_IDLE_TICKS)
    }
}

fn find_energy_near(creep: &Creep, _: StructureRampart, room: &Room, _: &[Creep]) -> Task {
    dropped_near(room, creep.pos())
        .or_else(|| full_container_near(creep.pos(), room))
        .or_else(|| {
            find_source_near(creep.pos(), room)
                .map(|source| Task::Harvest(creep.pos(), source.id()))
        })
        .unwrap_or(Task::Idle(1))
}

fn dropped_near(room: &Room, workplace: Position) -> Option<Task> {
    room.find(find::DROPPED_RESOURCES, None)
        .iter()
        .find(|resource| {
            resource.pos().is_near_to(workplace)
                && resource.resource_type() == ResourceType::Energy
                && resource.amount() > HANDYMAN_ENERGY_PICKUP_THRESHOLD
        })
        .map(|resource| Task::TakeResource(resource.id()))
}

fn full_container_near(workplace: Position, room: &Room) -> Option<Task> {
    room.find(find::STRUCTURES, None).iter().find_map(|s| match s {
        StructureObject::StructureContainer(c)
            if c.pos().is_equal_to(workplace)
                && c.store().get_used_capacity(Some(ResourceType::Energy)) > 1000 =>
        {
            Some(Task::TakeFromStructure(c.pos(), c.raw_id(), ResourceType::Energy, None))
        }
        _ => None,
    })
}

fn find_energy_or_source(room: &Room, workplace: Position) -> Task {
    debug!("startup handy is looking for energy");
    // check for energy on the ground of sufficient quantity to care about
    for resource in room.find(find::DROPPED_RESOURCES, None) {
        if resource.resource_type() == ResourceType::Energy
            && resource.amount() >= HANDYMAN_ENERGY_PICKUP_THRESHOLD
        {
            return Task::TakeResource(resource.id());
        }
    }

    for tomb in room.find(find::TOMBSTONES, None) {
        let energy_amount = tomb.store().get_used_capacity(Some(ResourceType::Energy));
        if energy_amount >= HANDYMAN_ENERGY_PICKUP_THRESHOLD {
            return Task::TakeFromStructure(tomb.pos(), tomb.raw_id(), ResourceType::Energy, None);
        }
    }

    // check structures - filtering for certain types, don't want
    // to have these taking from spawns or extensions!
    for structure in room.find(find::STRUCTURES, None) {
        let store = match &structure {
            StructureObject::StructureContainer(o) => o.store(),
            _ => {
                // we don't want to look at this!
                continue;
            }
        };

        if store.get_used_capacity(Some(ResourceType::Energy)) >= 1000 {
            return Task::TakeFromStructure(
                structure.pos(),
                structure.as_structure().raw_id(),
                ResourceType::Energy,
                None,
            );
        }
    }

    if let Some(source) = find_source_near(workplace, room) {
        if let Some(ctrl) = room.controller()
            && ctrl.pos().get_range_to(workplace) <= 3
        {
            Task::HarvestAndUpgrade(workplace, source.id(), ctrl.id())
        } else {
            Task::Harvest(workplace, source.id())
        }
    } else {
        Task::Idle(NO_TASK_IDLE_TICKS)
    }
}
