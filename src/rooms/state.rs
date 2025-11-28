use log::*;
use serde::{Serialize, Deserialize};
use screeps::{OrderType, PowerType, ResourceType, RoomName, game};
use std::{collections::{HashMap, HashSet}, hash::{Hash, Hasher}, iter::Iterator};
use ordered_float::OrderedFloat;

use crate::{
    rooms::state::{requests::Request, constructions::RoomPlan},
    units::{Memory, roles::Role}
};

pub mod requests;
pub mod constructions;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RoomState {
    #[serde(default)]
    pub spawns: Vec<Role>,
    #[serde(default)]
    pub requests: HashSet<Request>,
    // #[serde(default)]
    // pub perimetr: Vec<ObjectId<StructureRampart>>,
    #[serde(default)]
    pub plan: Option<RoomPlan>,
    #[serde(default = "HashMap::new")]
    pub farms: HashMap<RoomName, FarmInfo>,
    #[serde(default = "HashSet::new")]
    pub trades: HashSet<TradeData>,
    #[serde(skip)]
    pub intrusion: bool,
    #[serde(skip)]
    pub last_intrusion: u32,
    #[serde(default)]
    pub origin: bool, // no haulers are spawned, if not enough energy partially scale haulers
    #[serde(default)]
    pub pc_workplace: Option<(u8, u8)>,
    #[serde(default)]
    pub safe_place: Option<(u8, u8)>,
    #[serde(default = "HashSet::new")]
    pub powers: HashSet<PowerType>,
    #[serde(default = "HashMap::new")]
    pub boosts: HashMap<BoostReason, u32>,
}

impl RoomState {
    pub fn set_plan(&mut self, plan: RoomPlan) {
        if let Some(existed) = &mut self.plan {
            existed.add_cells(plan.planned_cells().into_iter());
        } else {
            self.plan = Some(plan)
        }
    }

    pub fn set_farm_plan(&mut self, name: RoomName, mut plan: RoomPlan) {
        self.farms.entry(name)
            .and_modify(|info| {
                if let Some(existed) = info.plan.take() {
                    plan.add_cells(existed.planned_cells().into_iter());
                }
                info.plan = Some(plan);
            });
    }

    pub fn add_to_spawn(&mut self, role: Role, times: usize) {
        info!("add {:?}: {} to spawn queue", role.clone(), times);
        for _ in 1..times + 1 {
            self.spawns.push(role.clone());
            // self.spawns.insert(role.clone());
        }
    }

    pub fn finish_farm(&mut self, farm: RoomName, with_central: Option<RoomName>) {
        debug!("finish_farm: {}, with_central: {:?}", farm, with_central);
        self.set_farm_for(farm, FarmStatus::Suspended);
        if let Some(central) = with_central {
            self.set_farm_for(central, FarmStatus::Suspended);
        }
    }

    pub fn begin_farm(&mut self, farm: RoomName, with_central: Option<RoomName>) {
        debug!("begin_farm: {}, with_central: {:?}", farm, with_central);
        self.set_farm_for(farm, FarmStatus::Building);
        if let Some(central) = with_central {
            self.set_farm_for(central, FarmStatus::Building);
        }
    }

    fn set_farm_for(&mut self, farm: RoomName, status: FarmStatus) {
        self.farms.entry(farm)
            .and_modify(|farm_room| { farm_room.update_status(status); })
            .or_default();
    }

    pub fn add_boost(&mut self, reason: BoostReason, time: u32) {
        self.boosts.entry(reason)
            .and_modify(|expire| {
                if *expire < time {
                    *expire = time;
                }
            })
            .or_insert(time);
    }

    pub fn find_roles<'a>(&'a self, role: &'a Role, creeps: &'a HashMap<String, Memory>) -> impl Iterator<Item = &'a Role> {
        self.in_spawn(role)
            .chain(creeps.values()
                .map(|mem| &mem.role)
                .filter(move |r| *r == role))
    }

    fn in_spawn<'a>(&'a self, role: &'a Role) -> impl Iterator<Item = &'a Role> {
        self.spawns
            .iter()
            .filter(move |future_creep| *future_creep == role)
    }

    pub fn update_expired_boosts(&mut self) {
        self.boosts.retain(|_, timeout| game::time() < *timeout);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FarmInfo {
    farm_status: FarmStatus,
    plan: Option<RoomPlan>,
}

impl FarmInfo {
    pub fn update_status(&mut self, status: FarmStatus) {
        self.farm_status = status;
    }

    pub fn plan(&self) -> Option<&RoomPlan> {
        self.plan.as_ref()
    }

    pub fn is_active(&self) -> bool { self.farm_status.is_active() }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub enum FarmStatus {
    Building,
    Spawning,
    #[default]
    Ready,
    Suspended
}

impl FarmStatus {
    fn is_active(&self) -> bool { !matches!(self, FarmStatus::Suspended) }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum BoostReason {
    Invasion,
    Upgrade,
    Repair,
    Dismantle,
    Caravan,
    Pvp,
    Carry,
}

impl BoostReason {
    pub fn value(&self) -> Vec<ResourceType> {
        match *self {
            BoostReason::Invasion => vec![ResourceType::CatalyzedUtriumAcid, ResourceType::CatalyzedLemergiumAlkalide],
            BoostReason::Carry => vec![ResourceType::CatalyzedKeaniumAcid],
            BoostReason::Upgrade => vec![ResourceType::CatalyzedGhodiumAcid],
            BoostReason::Repair => vec![ResourceType::CatalyzedLemergiumAcid],
            BoostReason::Dismantle => vec![ResourceType::CatalyzedZynthiumAcid],
            BoostReason::Caravan => vec![ResourceType::CatalyzedKeaniumAlkalide],
            BoostReason::Pvp => vec![ResourceType::CatalyzedGhodiumAlkalide, ResourceType::CatalyzedKeaniumAlkalide, ResourceType::CatalyzedLemergiumAlkalide, ResourceType::CatalyzedZynthiumAlkalide],
        }
    }
}

// #[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
// pub enum LabStatus {
//     Boost(ResourceType),
//     Input,
//     #[default]
//     Output
// }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq)]
pub struct TradeData {
    pub order_type: OrderType,
    pub resource: ResourceType,
    pub price: OrderedFloat<f64>,
    // pub former_price: Option<OrderedFloat<f64>>,
    pub amount: u32,
}

impl TradeData {
    pub fn new(order_type: OrderType, resource: ResourceType) -> Self {
        Self { order_type, resource, price: OrderedFloat::default(), amount: 0 }
    }

    pub fn with_price_and_amount(order_type: OrderType, resource: ResourceType, price: OrderedFloat<f64>, amount: u32) -> Self {
        Self { order_type, resource, price, amount }
    }
}

impl Hash for TradeData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.resource.hash(state);
    }
}

impl PartialEq for TradeData {
    fn eq(&self, other: &TradeData) -> bool {
        self.resource == other.resource
    }
}