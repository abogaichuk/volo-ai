use log::*;
use serde::{Deserialize, Serialize};
use screeps::{
    game::{self, map::get_room_linear_distance},
    raw_memory, ResourceType, RoomName,
};
use crate::{
    movement::Movement, rooms::{
        register_rooms,
        state::{
            FarmStatus, RoomState, requests::{
                CaravanData, DepositData, LRWData, PowerbankData, ProtectData, Request, RequestKind, TransferData, assignment::Assignment
            }
        },
        wrappers::claimed::Claimed,
    }, statistics::Statistic, units::{creeps::{CreepMemory, run_creeps}, power_creep::{PowerCreepMemory, run_power_creeps}, roles::Kind}, utils::constants::MAX_POWER_CAPACITY
};
use std::{collections::{HashMap, HashSet}, iter::once};
use js_sys::JsString;

pub mod events;
mod orders;

pub use events::ColonyEvent;
use crate::colony::orders::ColonyOrder;
use events::ColonyContext;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalState {
    #[serde(default)]
    pub rooms: HashMap<RoomName, RoomState>,
    #[serde(default)]
    pub creeps: HashMap<String, CreepMemory>,
    #[serde(default)]
    pub power_creeps: HashMap<String, PowerCreepMemory>,
    #[serde(skip, default = "game::time")]
    pub global_init_time: u32, // the tick when this state was created
    #[serde(default = "HashMap::new")]
    pub avoid_rooms: HashMap<RoomName, u32>,
    #[serde(default = "HashSet::new")]
    pub postponed_farms: HashSet<RoomName>, // postponed powerbank farm in rooms
    #[serde(default = "HashSet::new")]
    orders: HashSet<ColonyOrder>,
    #[serde(default)]
    pub statistic: Statistic,
    #[serde(default = "HashSet::new")]
    pub white_list: HashSet<String>,
    #[serde(default = "HashSet::new")]
    pub black_list: HashSet<String>,
}

impl Default for GlobalState {
    fn default() -> GlobalState {
        GlobalState {
            global_init_time: game::time(),
            rooms: HashMap::new(),
            creeps: HashMap::new(),
            power_creeps: HashMap::new(),
            avoid_rooms: HashMap::new(),
            postponed_farms: HashSet::new(),
            orders: HashSet::new(),
            statistic: Statistic::default(),
            white_list: HashSet::new(),
            black_list: HashSet::new()
        }
    }
}

impl GlobalState {
    pub fn run_tick(&mut self) {
        let orders = game::market::get_all_orders(None);
        let (mut homes, neutrals) = register_rooms(
            &mut self.rooms,
            &self.white_list
        );

        let mut events = Vec::new();
        for home in homes.values_mut() {
            let cpu_start = game::cpu::get_used();
            events.extend(home.run_shelter(&mut self.creeps, &orders));
            debug!("{} run_base for {} cpu!", home.name(), game::cpu::get_used() - cpu_start);
        }

        events.extend(neutrals.into_iter()
            .flat_map(|neutral| neutral.run_room()));

        let owned_rooms: Vec<RoomName> = homes.iter()
            .flat_map(|(room_name, home)| home.get_farms()
                .chain(once(*room_name)))
            .collect();

        let mut movement = Movement::new(&self.avoid_rooms, owned_rooms);
        //creeps are running after rooms to avoid case when creep has solved and removed a room request
        // but the room doesn't know it yet and creates a new one
        let cpu_start = game::cpu::get_used();
        run_power_creeps(&mut self.power_creeps, &mut homes, &mut movement);
        debug!("run_power_creeps {} cpu!", game::cpu::get_used() - cpu_start);

        let cpu_start = game::cpu::get_used();
        run_creeps(&mut self.creeps, &mut homes, &mut movement);
        debug!("finished run creeps {} cpu!", game::cpu::get_used() - cpu_start);

        movement.swap_move();

        let bases: HashMap<RoomName, Claimed> = homes.into_iter()
            .map(|(name, home)| (name, home.base()))
            .collect();

        let context = ColonyContext::new(movement, &bases);
        for event in events {
            event.assign(self, &context);
        }

        if game::time() % 100 == 0 {
            self.update_avoid_rooms();
            self.orders.retain(|order| game::time() < order.timeout());
            // self.update_statistics(Statistic::new(self, &bases));
        }
        self.gc();
    }

    pub fn begin_farm(&mut self, base: RoomName, farm: RoomName, with_central: Option<RoomName>) {
        debug!("begin_farm: {}, farm_room: {}, with_central: {:?}", base, farm, with_central);
        self.set_farm_for(base, farm, FarmStatus::Building);
        if let Some(central) = with_central {
            self.set_farm_for(base, central, FarmStatus::Building);
        }
    }

    pub fn finish_farm(&mut self, base: RoomName, farm: RoomName, with_central: Option<RoomName>) {
        debug!("finish_farm: {}, farm_room: {}, with_central: {:?}", base, farm, with_central);
        self.set_farm_for(base, farm, FarmStatus::Suspended);
        if let Some(central) = with_central {
            self.set_farm_for(base, central, FarmStatus::Suspended);
        }
    }

    fn set_farm_for(&mut self, base: RoomName, farm: RoomName, status: FarmStatus) {
        debug!("set_farm_for :{}, farm_room: {}", base, farm);
        self.rooms.entry(base)
            .and_modify(|room_state| {
                room_state.farms.entry(farm)
                    .and_modify(|farm_room| { farm_room.update_status(status); })
                    .or_default();
            });
    }

    fn add_request(&mut self, to: RoomName, request: Request) {
        debug!("add_request to :{}, request: {:?}", to, request);
        self.rooms.entry(to)
            .and_modify(|room_state| {
                room_state.requests.insert(request);
            });
    }

    fn update_avoid_rooms(&mut self) {
        let time = game::time();
        self.avoid_rooms.retain(|_, v| {
            *v > time
        });
    }

    fn update_statistics(&mut self, stats: Statistic) {
        self.statistic = stats;
    }

    pub fn load_or_default() -> GlobalState {
        let s = raw_memory::get().as_string().unwrap();
        info!("Raw memory: {s:?}");
        match serde_json::from_str(&s) {
            Ok(v) => {
                info!("v: {:?}", v);
                v
            },
            Err(e) => {
                error!("memory parse error, using default: {:?}", e);
                GlobalState::default()
            }
        }
    }

    pub fn write(&self) {
        debug!("Writing GameMemory to persistent memory");
        match serde_json::to_string(&self) {
            Ok(state) => {
                raw_memory::set(&JsString::from(state));
            }
            Err(e) => {
                warn!("memory write error: {:?}", e);
            }
        }
    }

    fn gc(&mut self) {
        self
            .creeps
            .retain(|name, mem| {
                if game::creeps().get(name.to_string()).is_some() {
                    true
                } else if !mem.respawned && mem.role.respawn_timeout(None).is_some() {
                    mem.respawned = true;
                    let _ = mem.role.get_home()
                        .map(|home| self.rooms.entry(*home)
                            .and_modify(|room_state|
                                room_state.add_to_spawn(mem.role.clone(), 1)));
                    false
                } else {
                    false
                }
            });
    }
}

fn prefered_room<'a, F, I>(
    target_room: RoomName,
    movement: &Movement,
    bases: I,
    accumulator: F,
) -> Option<(RoomName, usize)>
where
    I: IntoIterator<Item = &'a Claimed>,
    F: for<'b> FnMut(Option<(&'b Claimed, usize)>, (&'b Claimed, usize)) -> Option<(&'b Claimed, usize)>,
{
    bases.into_iter()
        .filter(|base|
            base.storage().is_some() &&
            get_room_linear_distance(base.get_name(), target_room, false) < 4)
        .filter_map(|base| {
            match game::map::find_route(base.get_name(), target_room, Some(movement.get_find_route_options())) {
                Ok(steps) => Some((base, steps.len())),
                _ => None
            }
        })
        .filter(|(_, distance)| *distance < 5)
        .fold(None, accumulator)
        .map(|(base, distance)| (base.get_name(), distance))
}

fn less_cga<'a>(
    first: Option<(&'a Claimed, usize)>,
    second: (&'a Claimed, usize),
) -> Option<(&'a Claimed, usize)> {
    if let Some(first_storage) = first.and_then(|base| base.0.storage()) {
        let f_cap = first_storage.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAcid));
        if let Some(second_storage) = second.0.storage() {
            let s_cap = second_storage.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAcid));
            if f_cap < s_cap { first } else { Some(second) }
        } else {
            first
        }
    } else if second.0.storage().is_some() {
        Some(second)
    } else {
        None
    }
}

fn less_power<'a>(
    first: Option<(&'a Claimed, usize)>,
    second: (&'a Claimed, usize),
) -> Option<(&'a Claimed, usize)> {
    if let Some(first_storage) = first.and_then(|(base, _)| base.storage()) {
        if let Some(second_storage) = second.0.storage() {
            let f_cap = first_storage.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAcid));
            let s_cap = second_storage.store().get_used_capacity(Some(ResourceType::Power));
            if s_cap >= MAX_POWER_CAPACITY || f_cap < s_cap {
                first
            } else {
                Some(second)
            }
        } else {
            first
        }
    } else if second.0.storage()
        .is_some_and(|storage| storage.store().get_used_capacity(Some(ResourceType::Power)) < MAX_POWER_CAPACITY)
    {
        Some(second)    
    } else {
        None
    }
}

fn most_money<'a>(
    first: Option<(&'a Claimed, usize)>,
    second: (&'a Claimed, usize),
) -> Option<(&'a Claimed, usize)> {
    if let Some(first_storage) = first.and_then(|(base, _)| base.storage()) {
        let f_cap = first_storage.store().get_used_capacity(Some(ResourceType::Energy));
        if let Some(second_storage) = second.0.storage() {
            let s_cap = second_storage.store().get_used_capacity(Some(ResourceType::Energy));
            if f_cap > s_cap { first } else { Some(second) }
        } else {
            first
        }
    } else if second.0.storage().is_some() {
        Some(second)
    } else {
        None
    }
}

fn most_ctrl_lvl<'a>(
    first: Option<(&'a Claimed, usize)>,
    second: (&'a Claimed, usize),
) -> Option<(&'a Claimed, usize)> {
    if let Some((first_base, _)) = first {
        if first_base.controller.level() > second.0.controller.level() {
            first
        } else if first_base.controller.level() < second.0.controller.level() {
            Some(second)
        } else if let Some(second_storage) = second.0.storage() {
            let first_storage = first_base.storage().expect("expect storage in first_base");

            let f_cap = first_storage.store().get_used_capacity(Some(ResourceType::Energy));
            let s_cap = second_storage.store().get_used_capacity(Some(ResourceType::Energy));
            if f_cap > s_cap { first } else { Some(second) }        
        } else {
            first
        }
    } else if second.0.storage().is_some() {
        Some(second)
    } else {
        None
    }
}
