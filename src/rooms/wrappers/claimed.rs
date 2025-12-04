use log::*;
use screeps::{
    ConstructionSite, Creep, Event, HasHits, HasId, HasPosition, HasStore, INVADER_USERNAME, MaybeHasId, Mineral, Nuke, Part, PowerCreep, RESOURCES_ALL, RawObjectId, Resource, ResourceType, Room, RoomName, RoomXY, SharedCreepProperties, Source, StructureContainer, StructureController, StructureExtension, StructureFactory, StructureLab, StructureNuker, StructureObject, StructureObserver, StructurePowerSpawn, StructureRoad, StructureSpawn, StructureStorage, StructureTerminal, StructureTower, StructureType, StructureWall, Tombstone, find, game
};
use smallvec::SmallVec;
use std::{cmp::min, collections::HashMap, iter::once};
use crate::{
    commons::{find_container_near_by, has_part, is_cpu_on_low}, resources::{self, Resources, RoomContext},
    rooms::{
        RoomEvent, RoomState, is_extractor, missed_buildings,
        state::{BoostReason, FarmInfo, constructions::{RoomPlan, RoomPlannerError},
        requests::{BodyPart, BuildData, CarryData, CreepHostile, PickupData,
            RepairData, Request, RequestKind, WithdrawData, assignment::Assignment}}, wrappers::{Fillable, claimed::structures::{labs::Labs, links::Links, ramparts::Ramparts}}
    },
    units::{
        creeps::CreepMemory,
        roles::{Role, combat::guard::Guard, miners::mineral_miner::MineralMiner, services::upgrader::Upgrader}
    },
    utils::constants::{
        MAX_CARRY_REQUEST_AMOUNT, MAX_WALL_HITS, MIN_PERIMETR_HITS, MY_ROOMS_PICKUP_RESOURCE_THRESHOLD
    }
};
use super::farm::Farm;

mod structures;

//todo implement prelude.rs
pub(crate) struct Claimed {
    pub(crate) room: Room,
    pub(crate) controller: StructureController,
    pub(crate) farms: Vec<Farm>, //todo move to shelter
    pub(crate) mineral: Mineral,
    pub(crate) sources: Vec<Source>,
    pub(crate) spawns: Vec<StructureSpawn>,
    pub(crate) extensions: Vec<StructureExtension>,
    pub(crate) towers: Vec<StructureTower>,
    pub(crate) storage: Option<StructureStorage>,
    pub(crate) links: Links,
    pub(crate) terminal: Option<StructureTerminal>,
    pub(crate) factory: Option<StructureFactory>,
    pub(crate) observer: Option<StructureObserver>,
    pub(crate) nuker: Option<StructureNuker>,
    pub(crate) power_spawn: Option<StructurePowerSpawn>,
    pub(crate) labs: Labs,
    pub(crate) ramparts: Ramparts,
    pub(crate) containers: Vec<StructureContainer>,
    pub(crate) roads: Vec<StructureRoad>,
    pub(crate) walls: Vec<StructureWall>,
    pub(crate) hostiles: Vec<Creep>,
    pub(crate) my_creeps: Vec<Creep>,
    pub(crate) my_pcreeps: Vec<PowerCreep>,
    pub(crate) nukes: Vec<Nuke>,
    pub(crate) tombs: Vec<Tombstone>,
    pub(crate) cs: Vec<ConstructionSite>,
    pub(crate) dropped: Vec<Resource>,
    pub(crate) resources: Resources,
    pub(crate) events: Vec<Event>
}

impl Claimed {
    pub(crate) fn new(room: Room, farms: Vec<Farm>, state: &RoomState) -> Self {
        let controller = room.controller().expect("expect controller in my Base");
        let mineral = room.find(find::MINERALS, None).remove(0);
        let sources = room.find(find::SOURCES, None);
        let hostiles = room.find(find::HOSTILE_CREEPS, None);
        let my_creeps = room.find(find::MY_CREEPS, None);
        let my_pcreeps = room.find(find::MY_POWER_CREEPS, None);
        let nukes = room.find(find::NUKES, None);
        let cs = room.find(find::CONSTRUCTION_SITES, None);
        let tombs = room.find(find::TOMBSTONES, None);
        let dropped = room.find(find::DROPPED_RESOURCES, None);
        let mut spawns = Vec::new();
        let mut extensions = Vec::new();
        let mut towers = Vec::new();
        let storage = room.storage();
        let mut links = Vec::new();
        let terminal = room.terminal();
        let mut factory = None;
        let mut observer = None;
        let mut nuker = None;
        let mut power_spawn = None;
        let mut labs = Vec::new();
        let mut ramparts = Vec::new();
        let mut containers = Vec::new();
        let mut roads = Vec::new();
        let mut walls = Vec::new();
        let events = room.get_event_log();

        for structure in room.find(find::STRUCTURES, None) {
            match structure {
                StructureObject::StructureTower(tower)
                    if tower.store().get_used_capacity(Some(ResourceType::Energy)) > 0 => towers.push(tower),
                StructureObject::StructureSpawn(s) => spawns.push(s),
                StructureObject::StructureExtension(e) => extensions.push(e),
                StructureObject::StructureFactory(f) => factory = Some(f),
                StructureObject::StructureRampart(r) => ramparts.push(r),
                StructureObject::StructureLink(link) => links.push(link),
                StructureObject::StructureLab(lab) => labs.push(lab),
                StructureObject::StructureContainer(c) => containers.push(c),
                StructureObject::StructureObserver(o) => observer = Some(o),
                StructureObject::StructurePowerSpawn(ps) => power_spawn = Some(ps),
                StructureObject::StructureNuker(n) => nuker = Some(n),
                StructureObject::StructureRoad(r) => roads.push(r),
                StructureObject::StructureWall(w) => walls.push(w),
                _ => {}
            }
        }

        let amounts = if game::time() % 100 == 0 {
            RESOURCES_ALL.iter()
                .map(|res| {
                    let mut total: u32 = 0;
                    if let Some(storage) = storage.as_ref() {
                        total += storage.store().get_used_capacity(Some(*res));
                    }
                    if let Some(terminal) = terminal.as_ref() {
                        total += terminal.store().get_used_capacity(Some(*res));
                    }
                    if let Some(factory) = factory.as_ref() {
                        total += factory.store().get_used_capacity(Some(*res));
                    }
                    for lab in labs.iter() {
                        total += lab.store().get_used_capacity(Some(*res));
                    }
                    (*res, total)
                })
                .collect()
        } else {
            HashMap::new()
        };

        Self {
            room,
            controller,
            farms,
            mineral,
            sources,
            spawns,
            extensions,
            towers,
            storage,
            links: Links::new(links, state.plan.as_ref()),
            terminal,
            factory,
            observer,
            nuker,
            power_spawn,
            labs: Labs::new(labs, state.plan.as_ref()),
            ramparts: Ramparts::new(ramparts, state.plan.as_ref()),
            containers,
            roads,
            walls,
            hostiles,
            my_creeps,
            my_pcreeps,
            nukes,
            tombs,
            cs,
            dropped,
            events,
            resources: Resources::new(amounts)
        }
    }

    fn plan_farm(&self, plan: &RoomPlan, farm_infos: &HashMap<RoomName, FarmInfo>) -> Result<HashMap<RoomName, RoomPlan>, RoomPlannerError> {
        if let Some((name, _)) = farm_infos.iter()
            .find(|(_, info)| info.plan().is_none())
        {
            let farm = self.farms.iter().find(|f| f.get_name() == *name)
                .ok_or(RoomPlannerError::UnreachableRoom)?;

            let plans = farm_infos.iter()
                .filter_map(|(name, info)| info.plan().map(|plan| (*name, plan)))
                .chain(once((self.get_name(), plan)))
                .collect();
            
            farm.plan_room(plans)
        } else {
            Err(RoomPlannerError::AlreadyCreated)
        }
    }

    fn constructions_check(&self, memory: &RoomState) -> Option<RoomEvent> {
        if let Some(plan) = &memory.plan {
            match self.plan_farm(plan, &memory.farms) {
                Ok(plans) => Some(RoomEvent::AddPlans(plans)),
                Err(err) => {
                    match err {
                        RoomPlannerError::AlreadyCreated => {
                            //todo create cs partially and in specific order
                            let buildings: HashMap<RoomXY, StructureType> = missed_buildings(self.get_name(), plan)
                                .collect();
                            let cpu_start = game::cpu::get_used();
                            if !buildings.is_empty() {
                                buildings.into_iter()
                                    .for_each(|(xy, str_type)| {
                                        // info!("{} place cs: {} at {}", self.get_name(), str_type, xy);
                                        match self.room.create_construction_site(
                                            xy.x.u8(),
                                            xy.y.u8(),
                                            str_type,
                                            None)
                                        {
                                            Ok(_) => {},
                                            Err(err) => {
                                                error!("{} can't create cs: {}, at: {}, err: {:?}", self.get_name(), str_type, xy, err);
                                            }
                                        }
                                    });
                                let cpu_used = game::cpu::get_used() - cpu_start;
                                info!("{} created cs cpu used: {}", self.get_name(), cpu_used);
                                None
                            } else if plan.built_lvl() < self.controller.level() {
                                Some(RoomEvent::BuiltAll)
                            } else {
                                None
                            }
                        },
                        e => {
                            error!("{} creation plan error: {}", self.get_name(), e);
                            None
                        }
                    }
                }
            }
        } else if !is_cpu_on_low() {
            match self.generate_plan() {
                Ok(plan) => Some(RoomEvent::Plan(plan)),
                Err(err) => {
                    error!("{}", err);
                    None
                }
            }
        } else {
            None
        }
    }

    //todo withdrawrequest instead of carry
    pub fn unload<T>(&self, obj: &T, allowed: &[ResourceType]) -> Option<RoomEvent>
        where T: HasStore + HasId
    {
        self.storage.as_ref()
            .filter(|storage| storage.store().get_free_capacity(None) > 10000)
            .and_then(|storage| {
                obj.store().store_types()
                    .into_iter()
                    .find_map(|resource| {
                        if !allowed.contains(&resource) {
                            let amount = obj.store().get_used_capacity(Some(resource));
                            if resource != ResourceType::Energy || amount > 15000 {
                                return Some(RoomEvent::Request(Request::new(
                                    RequestKind::Carry(CarryData::new(
                                        obj.raw_id(),
                                        storage.raw_id(),
                                        resource,
                                        min(amount, MAX_CARRY_REQUEST_AMOUNT))),
                                    Assignment::Single(None))));
                            }
                        }
                        None
                    })
            })
    }

    pub fn supply_resources(&self, to: RawObjectId, resource: ResourceType, amount: u32) -> Option<RoomEvent> {
        self.storage()
            .and_then(|storage| {
                let storage_capacity = storage.store().get_used_capacity(Some(resource));
                if storage_capacity < amount || ResourceType::Energy == resource && storage_capacity < 25000 {
                    None
                } else {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Carry(CarryData::new(
                            storage.raw_id(),
                            to,
                            resource,
                            amount)),
                        Assignment::Single(None))))
                }
            })
    }

    pub fn time_based_events<'a>(
        &'a self,
        room_memory: &'a RoomState,
        creeps: &'a HashMap<String, CreepMemory>) -> impl Iterator<Item = RoomEvent> + 'a
    {
        (game::time() % 100 == 0)
            .then(|| {
                once(RoomEvent::RetainBoosts)
                    .chain(self.manage_mineral_miner(room_memory, creeps))
                    .chain(self.manage_controller(room_memory, creeps))
                    .chain(self.resource_handler())
                    .chain(self.constructions_check(room_memory))
            })
            .into_iter().flatten()
    }

    fn resource_handler(&self) -> impl Iterator<Item = RoomEvent> + use<'_> {
        //todo create resource handler here because:
        // 1. the same time check
        // 2. easy to create RoomStats because creeps len is here
        // 3. easy pass to colony by throwing colonyevent
        if self.controller.level() > 6 {
            let context = RoomContext::new(
                self.controller.level(),
                self.terminal.as_ref().map(|t| t.raw_id()),
                self.storage.as_ref().map(|s| s.raw_id()),
                self.factory.as_ref().map(|f| f.level()).unwrap_or_default());
            Some(self.resources.events(context))
        } else {
            None
        }.into_iter().flatten()
    }

    pub fn get_name(&self) -> RoomName {
        self.room.name()
    }

    pub fn get_farms(&self) -> &[Farm] {
        &self.farms
    }

    pub fn all_minerals(&self) -> impl Iterator<Item = &Mineral> {
        once(&self.mineral)
            .chain(self.farms.iter().filter_map(|farm| farm.mineral.as_ref()))
    }

    pub fn all_sources(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter()
            .chain(self.farms.iter().flat_map(|farm| farm.sources.iter()))
    }

    pub fn storage(&self) -> Option<&StructureStorage> {
        self.storage.as_ref()
    }

    pub fn terminal(&self) -> Option<&StructureTerminal> {
        self.terminal.as_ref()
    }

    pub fn factory(&self) -> Option<&StructureFactory> {
        self.factory.as_ref()
    }

    pub fn production_labs(&self) -> (&[StructureLab], &[StructureLab]) {
        (self.labs.inputs(), self.labs.outputs())
    }

    pub fn energy_available(&self) -> u32 {
        self.room.energy_available()
    }

    pub fn energy_capacity_available(&self) -> u32 {
        self.room.energy_capacity_available()
    }

    pub fn resource_amount(&self, resource: ResourceType) -> u32 {
        self.storage()
            .map(|storage| storage.store().get_used_capacity(Some(resource)))
            .and_then(|in_storage| self.terminal()
                .map(|terminal| terminal.store().get_used_capacity(Some(resource)) + in_storage))
                .unwrap_or_default()
    }

    //todo the Box requires cloning, consider to avoid cloning by adding new lifetime to Task -> CreepMemory -> Unit
    pub(crate) fn closest_empty_structure(&self, to: &dyn HasPosition) -> Option<Box<dyn Fillable>> {
        self.extensions.iter()
            .filter(|e| e.store().get_free_capacity(Some(ResourceType::Energy)) > 0).cloned()
            .map(|e| Box::new(e) as Box<dyn Fillable>)
            .chain(self.towers.iter()
                .filter(|t| t.store().get_free_capacity(Some(ResourceType::Energy)) > 0).cloned()
                .map(|t| Box::new(t) as Box<dyn Fillable>))
            .chain(self.spawns.iter()
                .filter(|s| s.store().get_free_capacity(Some(ResourceType::Energy)) > 0).cloned()
                .map(|s| Box::new(s) as Box<dyn Fillable>))
            .min_by_key(|f| to.pos().get_range_to(f.position()))
    }

    pub fn build_requests(&self) -> Vec<Request> {
        self.cs
            .iter()
            .filter(|cs| cs.my())
            .map(move |cs| Request::new(
                RequestKind::Build(BuildData::new(cs.try_id(), cs.pos())),
                Assignment::Single(None)))
            .collect()
    }

    pub fn pickup_requests(&self) -> Vec<Request> {
        self.dropped
            .iter()
            .filter_map(move |resource| {
                if (resource.resource_type() == ResourceType::Energy && resource.amount() > MY_ROOMS_PICKUP_RESOURCE_THRESHOLD)
                    || (resource.resource_type() != ResourceType::Energy && resource.amount() >= 50)
                {
                    Some(Request::new(
                        RequestKind::Pickup(PickupData::new(resource.id())),
                        Assignment::Single(None)))
                    // Some(Request::Pickup(PickupRequest::new(resource.id())))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn tomb_requests(&self) -> Vec<Request> {
        self.tombs
            .iter()
            .filter(|tomb| tomb.store().get_used_capacity(None) > 650)
            .map(move |tomb| Request::new(
                    RequestKind::Withdraw(WithdrawData::new(
                        tomb.id().into(),
                        tomb.pos(),
                        tomb.store().store_types().into_iter().map(|res| (res, None)).collect())),
                    Assignment::Single(None)))
            .collect()
    }

    //spawn mineral miner if needed, he does suicide when finished his job
    fn manage_mineral_miner<'a>(
        &'a self,
        room_memory: &'a RoomState,
        creeps: &'a HashMap<String, CreepMemory>,
    ) -> Option<RoomEvent> {
        if self.mineral.ticks_to_regeneration().is_none() && is_extractor(&self.mineral) {
            if let Some(container) =
                find_container_near_by(&self.mineral.pos(), 1, &[StructureType::Container])
            {
                let role =
                    Role::MineralMiner(MineralMiner::new(Some(container.pos()), Some(self.get_name())));

                if room_memory.find_roles(&role, creeps).next().is_none() {
                    return Some(RoomEvent::Spawn(role, 1));
                }
            }
        }
        None
    }

    fn manage_controller(
        &self,
        room_memory: &RoomState,
        creeps: &HashMap<String, CreepMemory>) -> impl Iterator<Item = RoomEvent>
    {
        self.storage()
            .map(|storage| {
                let boost_amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAcid));

                if boost_amount > 500 && !room_memory.boosts.contains_key(&BoostReason::Upgrade) {
                    Some(RoomEvent::AddBoost(BoostReason::Upgrade, 1500))
                } else {
                    None
                }.into_iter()
                .chain(self.manage_upgraders(storage, room_memory, creeps))
            })
            .into_iter().flatten()
    }

    fn manage_upgraders(
        &self,
        storage: &StructureStorage,
        room_memory: &RoomState,
        creeps: &HashMap<String, CreepMemory>) -> Option<RoomEvent>
    {
        let upgrader = Role::Upgrader(Upgrader::new(Some(self.get_name())));
        let is_alive = room_memory.find_roles(&upgrader, creeps).next().is_some();
        let energy_amount = storage.store().get_used_capacity(Some(ResourceType::Energy));

        if self.controller.level() == 8 && energy_amount > 250000 && !is_alive {
            Some(RoomEvent::Spawn(upgrader, 1))
        } else if self.controller.level() == 8 && energy_amount < 150000 {
            Some(RoomEvent::CancelRespawn(upgrader))
        } else if energy_amount > 150000 && !is_alive {
            Some(RoomEvent::Spawn(upgrader, 1))
        } else if energy_amount < 15000 {
            Some(RoomEvent::CancelRespawn(upgrader))
        } else {
            None
        }
    }

    pub fn repair_roads(&self, plan: Option<&RoomPlan>) -> impl Iterator<Item = Request> {
        let mut requests = Vec::new();
        if let Some(plan) = plan {
            let planned_roads = plan.roads();
            for road in &self.roads {
                if road.hits() < (road.hits_max() as f32 * 0.5) as u32 && planned_roads.contains(&road.pos().xy()) {
                    let attempts = if road.hits_max() == 5000 { 2 } else { 5 };
                    requests.push(Request::new(
                        RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                            road.id().into_type(),
                            road.pos(),
                            attempts,
                            road.hits())),
                        Assignment::Single(None)));
                }
            }
        }
        requests.into_iter()
    }

    pub fn repair_walls(&self) -> impl Iterator<Item = Request> {
        let mut requests = Vec::new();

        for wall in &self.walls {
            if wall.hits() < MAX_WALL_HITS {
                requests.push(Request::new(
                    RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                        wall.id().into_type(),
                        wall.pos(),
                        5,
                        wall.hits())),
                    Assignment::Single(None)));
            }
        }
        requests.into_iter()
    }

    //todo logic close to logic for towers
    pub fn security_check(&self, room_memory: &RoomState, creeps: &HashMap<String, CreepMemory>) -> impl Iterator<Item = RoomEvent> {
        self.invasion_check(room_memory, creeps).into_iter()
            .chain(self.perimetr_check())
            .chain((!self.nukes.is_empty()).then(|| RoomEvent::NukeFalling))
    }

    pub fn invasion_check(&self, room_memory: &RoomState, creeps: &HashMap<String, CreepMemory>) -> SmallVec<[RoomEvent; 4]> {
        let mut events: SmallVec<[RoomEvent; 4]> = SmallVec::new();
        match self.controller.level() {
            ..=3 => {
                //any parts
                if self.hostiles.iter()
                    .any(|hostile| hostile.owner().username() != INVADER_USERNAME &&
                        has_part(&[Part::Attack, Part::RangedAttack, Part::Claim], hostile, false))
                {
                    events.push(RoomEvent::Intrusion(Some(format!("{} Invasion!", self.get_name()))));
                    let _ = self.controller.activate_safe_mode();
                }
            }
            4..=6 => {
                if !find_player_boosted_creeps(&self.hostiles).is_empty() && self.towers.is_empty() {
                    let _ = self.controller.activate_safe_mode();
                }
            }
            _ => {
                let boosted_enemies = find_player_boosted_creeps(&self.hostiles);
            
                if !boosted_enemies.is_empty() && !self.spawns.is_empty() {
                    events.push(RoomEvent::AddBoost(BoostReason::Invasion, 5000));
                    let guard = Role::Guard(Guard::new(Some(self.get_name())));
            
                    let alive_number = room_memory.find_roles(&guard, creeps).count();
                    let to_spawn = match boosted_enemies.len() {
                            1 | 2 if alive_number == 0 => 1,
                            3 | 4 if alive_number < 2 => 2 - alive_number,
                            5 .. if alive_number < 3 => 3 - alive_number,
                            _ => { 0 }
                    };
                    info!("{} alive guards: {} boosted_enemies: {}, to spawn: {}", self.get_name(), alive_number, boosted_enemies.len(), to_spawn);
                    events.push(RoomEvent::Spawn(guard, to_spawn));
                    events.push(RoomEvent::Intrusion(Some(format!("Boosted player invasion in room {}!!", self.get_name()))));

                } else if room_memory.intrusion && room_memory.last_intrusion + 100 < game::time() {
                    events.push(RoomEvent::Intrusion(None));
                }
            }
        }
        events
    }
    
    fn perimetr_check(&self) -> Option<RoomEvent> {
        (self.ramparts.perimeter()
            .any(|rampart| rampart.hits() < MIN_PERIMETR_HITS))
                .then(|| RoomEvent::ActivateSafeMode("Perimeter out of order! Enabling safe mode!".to_string()))
    }
}

fn find_player_boosted_creeps(enemies: &[Creep]) -> Vec<CreepHostile> {
    enemies.iter()
        .filter(|creep| {
            creep.owner().username() != INVADER_USERNAME && creep.body().iter()
                .any(|body_part| body_part.boost().is_some())
        })
        .map(|hostile| {
            CreepHostile {
                name: hostile.name(),
                owner: hostile.owner().username(),
                ticks_to_live: hostile.ticks_to_live(),
                parts: hostile.body().iter()
                    .map(|bodypart| BodyPart {
                        boosted: bodypart.boost().is_some(),
                        part: bodypart.part(),
                        hits: bodypart.hits()
                    }).collect()
            }
        }).collect()
}
