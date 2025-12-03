use std::{collections::HashMap};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use screeps::{game, HasHits, ObjectId, ResourceType, Room, RoomName, StructureController, StructureRampart, RESOURCES_ALL};
use crate::{GlobalState, rooms::{state::RoomState, wrappers::claimed::Claimed}};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Statistic {
    #[serde(default = "game::time")]
    pub tick: u32,
    #[serde(default)]
    pub cpu_bucket: i32,
    #[serde(default)]
    pub cpu_limit: u32,
    #[serde(default)]
    pub cpu_used: f64,
    #[serde(default)]
    pub last_restart: u32,
    #[serde(default = "Vec::new")]
    pub rooms: Vec<RoomStats>
}

impl Statistic {
    pub(crate) fn new(state: &GlobalState, bases: &HashMap<RoomName, Claimed>) -> Self {
        let rooms = state.rooms.iter()
            .map(|(room_name, room_memory)| {
                let creeps_number = state.creeps.iter()
                    .filter(|(_, memory)| memory.role.get_home()
                        .is_some_and(|home| home == room_name))
                    .count();
                let room = game::rooms().get(*room_name).expect("expect room is valid");
                RoomStats::new(&room, room_memory, creeps_number)
            })
            .collect();

        Statistic {
            tick: game::time(),
            cpu_bucket: game::cpu::bucket(),
            cpu_limit: game::cpu::limit(),
            cpu_used: game::cpu::get_used(),
            last_restart: state.global_init_time,
            rooms,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoomStats {
    name: RoomName,
    #[serde(default)]
    controller: ControllerStats,
    #[serde(default)]
    energy_in_use: u32,
    #[serde(default)]
    energy_capacity: u32,
    // resources: HashMap<ResourceType, u32>,
    // #[serde(default)]
    // perimetr: Perimetr,
    #[serde(default)]
    storage_used_capacity: Option<u32>,
    #[serde(default)]
    terminal_used_capacity: Option<u32>,
    #[serde(default)]
    requests: usize,
    #[serde(default)]
    creeps_number: usize,
    #[serde(default)]
    last_intrusion: u32,
}

impl RoomStats {
    pub fn new(room: &Room, room_memory: &RoomState, creeps_number: usize) -> Self {
        let controller = room.controller().expect("expect controller in claimed room");
        
        Self {
            name: room.name(),
            controller: ControllerStats::new(controller),
            energy_in_use: room.energy_available(),
            energy_capacity: room.energy_capacity_available(),
            // resources: RESOURCES_ALL
            //     .iter()
            //     .map(|resource| {
            //         let amount = get_resource_amount(room, *resource);
            //         (*resource, amount)
            //     })
            //     .filter(|(_, amount)| *amount > 100)
            //     .collect(),
            storage_used_capacity: room.storage().map(|storage| storage.store().get_used_capacity(None)),
            terminal_used_capacity: room.terminal().map(|terminal| terminal.store().get_used_capacity(None)),
            requests: room_memory.requests.len(),
            creeps_number,
            last_intrusion: room_memory.last_intrusion,
            // perimetr: Perimetr::new(&room_memory.perimetr)
        }
    }
}

fn get_resource_amount(room: &Room, resource: ResourceType) -> u32 {
    room.storage()
        .map(|storage| storage.store().get_used_capacity(Some(resource)))
        .unwrap_or_default()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControllerStats {
    level: u8,
    ticks_to_downgrade: Option<u32>,
    progress: Option<u32>
}

impl ControllerStats {
    pub fn new(controller: StructureController) -> Self {
        Self {
            level: controller.level(),
            ticks_to_downgrade: controller.ticks_to_downgrade(),
            progress: controller.progress()
        }
    }
}

// Memory.rooms[''].perimetr = ['']
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Perimetr {
    #[serde(default)]
    ramparts_number: usize,
    #[serde(default)]
    min_hits: u32,
    #[serde(default, serialize_with = "serialize_as_zero", deserialize_with = "deserialize_null_to_zero")]
    average_hits: f32
}

fn deserialize_null_to_zero<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<f32>::deserialize(deserializer)?.unwrap_or(0.0))
}

fn serialize_as_zero<S>(_: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f32(0.0)
}

impl Default for Perimetr {
    fn default() -> Self {
        Perimetr {
            ramparts_number: 0,
            min_hits: 0,
            average_hits: 0.,
        }
    }
}

impl Perimetr {
    pub fn new(perimetr: &[ObjectId<StructureRampart>]) -> Self {
        let mut ramparts_number = 0;
        let mut min_hits = 300000000;
        let mut hits_sum = 0;

        for rampart in perimetr.iter()
            .filter_map(|id| id.resolve())
        {
            ramparts_number += 1;
            hits_sum += rampart.hits();
            if rampart.hits() < min_hits {
                min_hits = rampart.hits();
            }
        }

        Self {
            ramparts_number,
            min_hits,
            average_hits: hits_sum as f32 / ramparts_number as f32
        }
    }
}