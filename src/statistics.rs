use std::collections::HashMap;
use serde::{Serialize, Deserialize, Serializer};
use screeps::{game, HasHits, ResourceType, RoomName, StructureController, StructureRampart};
use crate::rooms::wrappers::claimed::Claimed;

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
    #[serde(default = "HashMap::new")]
    pub rooms: HashMap<RoomName, RoomStats>
}

impl Statistic {
    pub fn update(&mut self, name: RoomName, stats: RoomStats) -> Option<RoomStats> {
        if self.tick < game::time() {
            self.tick = game::time();
            self.cpu_bucket = game::cpu::bucket();
            self.cpu_limit = game::cpu::limit();
            self.cpu_used = game::cpu::get_used();
        }
        self.rooms.insert(name, stats)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoomStats {
    controller: ControllerStats,
    #[serde(default)]
    energy_in_use: u32,
    #[serde(default)]
    energy_capacity: u32,
    #[serde(default)]
    resources: HashMap<ResourceType, u32>,
    #[serde(default)]
    perimetr: Perimetr,
    #[serde(default)]
    storage_used_capacity: Option<u32>,
    #[serde(default)]
    terminal_used_capacity: Option<u32>,
    #[serde(default)]
    requests: usize,
    #[serde(default)]
    creeps: usize,
    #[serde(default)]
    last_intrusion: u32,
}

impl RoomStats {
    pub(crate) fn new(
        base: &Claimed,
        requests: usize,
        last_intrusion: u32,
        creeps: usize) -> Self
    {        
        Self {
            controller: ControllerStats::new(&base.controller),
            energy_in_use: base.energy_available(),
            energy_capacity: base.energy_capacity_available(),
            resources: base.resources.all().clone(),
            storage_used_capacity: base.storage().map(|storage| storage.store().get_used_capacity(None)),
            terminal_used_capacity: base.terminal().map(|terminal| terminal.store().get_used_capacity(None)),
            last_intrusion,
            requests,
            creeps,
            perimetr: Perimetr::new(base.ramparts.perimeter())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControllerStats {
    level: u8,
    ticks_to_downgrade: Option<u32>,
    progress: Option<u32>
}

impl ControllerStats {
    pub fn new(controller: &StructureController) -> Self {
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
    min: u32,
    #[serde(default)]
    max: u32,
    #[serde(default, serialize_with = "serialize_f64")]
    average: f64
}

fn serialize_f64<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = (value * 1000.0).round() / 1000.0; // Round to 3 decimal places
    serializer.serialize_f64(rounded)
}

impl Default for Perimetr {
    fn default() -> Self {
        Perimetr {
            min: 0,
            max: 0,
            average: 0.,
        }
    }
}

impl Perimetr {
    pub fn new<'a>(perimetr: impl Iterator<Item = &'a StructureRampart>) -> Self {
        let mut len = 0;
        let mut sum = 0;
        let mut max = 0;
        let mut min = u32::MAX;

        for rampart in perimetr {
            len += 1;

            let hits = rampart.hits();
            sum += hits;

            if max < hits {
                max = hits;
            } else if min > hits {
                min = hits;
            }
        }

        let average = sum as f64 / len as f64;
        Self { min, max, average }
    }
}