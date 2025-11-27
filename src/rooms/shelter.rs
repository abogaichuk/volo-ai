use log::*;
use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use screeps::{
    game, game::market::Order, OrderType, ResourceType, RoomName, Room, Position,
    PowerType, StructureRampart, StructureLink, StructureStorage, StructureController,
    StructureLab, StructureTerminal, StructurePowerSpawn, StructureFactory, HasHits,
    Mineral, Source, ObjectId,  RawObjectId, Creep, Part, HasStore, HasId, HasPosition,
    Effect, EffectType, RoomObjectProperties, StructureSpawn, StructureTower
};
use crate::{
    colony::ColonyEvent, commons::find_roles,
    rooms::{
        RoomEvent, state::{RoomState, TradeData,
            requests::{CreepHostile, DefendData, Request, RequestKind, assignment::Assignment}
        },
        wrappers::{claimed::Claimed, farm::Farm}
    },
    units::{Memory, roles::Role}
};

pub struct Shelter<'s> {
    pub(crate) base: Claimed,
    pub(crate) state: &'s mut RoomState,
    pub(crate) white_list: &'s HashSet<String>
}

impl <'s> Shelter<'s> {
    pub(crate) fn new(
        base_room: Room,
        farms: Vec<Farm>,
        state: &'s mut RoomState,
        white_list: &'s HashSet<String>
    ) -> Self {
        Shelter { base: Claimed::new(base_room, farms, state), state, white_list }
    }

    pub fn run_shelter(&mut self, creeps: &mut HashMap<String, Memory>, orders: &[Order]) -> Vec<ColonyEvent> {
        //     //todo proportial perimetr security check
        //     //todo room_memory for requests excess, only room_memory spawns in use for dismantle or combats and room_memory.boost for factory
        //     //todo Request as updateable?
        //     //find better abstraction for request handler, sequential\parallel or implement task runner for factory, terminal and labs

        self.base.run_towers(&self.state.perimetr);
        let security_events = self.base.security_check(self.state, creeps);

        self.base.run_links();
        self.base.run_observer();

        let requests: HashSet<Request> = self.state.requests.drain().collect();

        // lab, terminal and factory switch request status to InProgress only
        // because only one request can be correctly handled at one tick
        let manufacture_events = self.base.run_labs(&requests, &self.state.boosts)
            .chain(self.base.run_factory(&requests, self.state))
            .chain(self.base.run_terminal(&requests, self.state, orders));

        let mut events: Vec<RoomEvent> = requests.into_iter()
            .chain(self.base.run_ramparts())
            .chain(self.base.run_power())
            .chain(self.base.run_nuker())
            .chain(self.base.run_containers())
            .chain(self.base.repair_roads(self.state.plan.as_ref()))
            .chain(self.base.repair_walls())
            .chain(self.base.tomb_requests())
            .chain(self.base.pickup_requests())
            .chain(self.base.build_requests())
            .flat_map(|request| request.handle(self, creeps))
            .chain(manufacture_events)
            .chain(security_events)
            .chain(self.base.run_spawns(self.state))
            .chain(self.base.time_based_events(self.state, creeps))
            .collect();
        
        events.extend(self.base.farms.iter()
            .flat_map(|farm| farm.run_farm(self.state.farms.entry(farm.get_name()).or_default())));

        let mut colony_events = Vec::new();
        for event in events {
            match event {
                RoomEvent::Request(request) => {
                    self.state.requests.insert(request);
                },
                RoomEvent::Defend(farm_name, hostiles) => {
                    let hostiles: Vec<CreepHostile> = hostiles.into_iter()
                        .filter(|ch| !self.white_list.contains(&ch.name))
                        .collect();
                    self.state.requests.insert(Request::new(
                        RequestKind::Defend(DefendData::with_hostiles(farm_name, hostiles)),
                        Assignment::Multi(HashSet::new())));
                },
                RoomEvent::ReplaceRequest(request) => {
                    self.state.requests.replace(request);
                },
                RoomEvent::Spawned(name, role, index) => {
                    self.state.spawns.remove(index);
                    creeps.insert(name, Memory::new(role));
                }
                RoomEvent::Spawn(role, times) => {
                    self.state.add_to_spawn(role, times);
                }
                RoomEvent::MayBeSpawn(mut role) => {
                    role.set_home(self.name());
                    let alive_number = find_roles(&role, &self.state.spawns, creeps);
                    if alive_number == 0 {
                        self.state.add_to_spawn(role, 1);
                    }
                }
                RoomEvent::CancelRespawn(role) => {
                    creeps.iter_mut()
                        .for_each(|creep| {
                            if creep.1.role == role {
                                creep.1.respawned = true;
                            }
                        });
                }
                RoomEvent::AddPower(power) => {
                    self.state.powers.insert(power);
                }
                RoomEvent::DeletePower(power) => {
                    self.state.powers.remove(&power);
                }
                RoomEvent::AddBoost(reason, timeout) => {
                    self.state.add_boost(reason, game::time() + timeout);
                }
                RoomEvent::RetainBoosts => {
                    self.state.update_expired_boosts();
                }
                RoomEvent::ReplaceCell(cell) => {
                    if let Some(plan) = self.state.plan.as_mut() {
                        plan.replace_cell(cell);
                    } else {
                        error!("{} no plan found!", self.name());
                    }
                }
                RoomEvent::Avoid(room_name, timeout) => {
                    colony_events.push(ColonyEvent::AvoidRoom(room_name, timeout));
                },
                RoomEvent::StopFarm(room_name, with_room) => {
                    self.state.finish_farm(room_name, with_room);
                },
                RoomEvent::StartFarm(room_name, with_room) => {
                    self.state.begin_farm(room_name, with_room);
                },
                RoomEvent::Lack(res, amount) => {
                    colony_events.push(ColonyEvent::Lack(self.name(), res, amount));
                },
                RoomEvent::Excess(res, amount) => {
                    colony_events.push(ColonyEvent::Excess(self.name(), res, amount));
                },
                RoomEvent::Sell(order_id, resource, amount) => {
                    if let Some(mut trade) = self.state.trades.take(&TradeData::new(OrderType::Sell, resource)) {
                        match game::market::deal(&order_id, amount, Some(self.name())) {
                            Ok(_) => {
                                trade.amount -= amount;
                                self.state.trades.insert(trade);
                                info!("{} sell: {}", self.name(), resource);
                            },
                            Err(err) => {
                                error!("sell trade error: {:?}", err);
                            }
                        }
                    } else {
                        error!("{} not found trade {:?} resource {}", self.name(), OrderType::Sell, resource);
                    }
                },
                RoomEvent::Buy(order_id, resource, amount) => {
                    if let Some(mut trade) = self.state.trades.take(&TradeData::new(OrderType::Buy, resource)) {
                        match game::market::deal(&order_id, amount, Some(self.name())) {
                            Ok(_) => {
                                trade.amount -= amount;
                                self.state.trades.insert(trade);
                                info!("{} buy: {}", self.name(), resource);
                            },
                            Err(err) => {
                                error!("buy trade error: {:?}", err);
                            }
                        }
                    } else {
                        error!("{} not found trade {:?} resource {}", self.name(), OrderType::Sell, resource);
                    }
                }
                RoomEvent::Intrusion(message) => {
                    if let Some(message) = message {
                        self.state.intrusion = true;
                        self.state.last_intrusion = game::time();
                        colony_events.push(ColonyEvent::Notify(message, Some(30)));
                    } else {
                        self.state.intrusion = false
                    }
                }
                RoomEvent::NukeFalling => {
                    let land_time = self.base.nukes.iter().map(|nuke| nuke.time_to_land()).min();
                    let message = format!("Nuke is launched to room {}, splash in: {:?}", self.name(), land_time);
                    colony_events.push(ColonyEvent::Notify(message, Some(30)));
                }
                RoomEvent::AddPlans(plans) => {
                    for (name, additional) in plans {
                        // info!("{} additional plan: {:?}", name, additional.planned_cells());
                        if name == self.name() {
                            self.state.set_plan(additional);
                        } else {
                            self.state.set_farm_plan(name, additional);
                        }
                    }
                }
                RoomEvent::Plan(plan) => {
                    info!("{} construction plan created!", self.name());
                    self.state.plan = Some(plan);
                }
                RoomEvent::BuiltAll => {
                    if let Some(mut plan) = self.state.plan.take() {
                        plan.increment_lvl();
                        self.state.plan = Some(plan);
                    }
                }
                RoomEvent::BlackList(username) => {
                    colony_events.push(ColonyEvent::BlackList(username));
                }
                RoomEvent::ActivateSafeMode(message) => {
                    // let _ = self.controller.activate_safe_mode();
                    warn!("{} activate safe mode!!", self.name());
                    colony_events.push(ColonyEvent::Notify(message, Some(30)));
                }
                // RoomEvent::Sos => {
                //     warn!("room event sos is not implemented yet!");
                //     //todo colony help me
                // }
            };
        }
        colony_events
    }

    pub fn name(&self) -> RoomName {
        self.base.get_name()
    }

    pub fn spawn_queue(&self) -> &[Role] {
        &self.state.spawns
    }

    pub fn get_farms(&self) -> impl Iterator<Item = RoomName> + use<'_> {
        self.base.get_farms().iter().map(|farm| farm.get_name())
    }

    pub(crate) fn base(self) -> Claimed {
        self.base
    }

    pub fn room(&self) -> &Room {
        &self.base.room
    }

    pub fn invasion(&self) -> bool {
        self.state.intrusion
    }

    pub fn is_power_enabled(&self, power: &PowerType) -> bool {
        self.state.powers.contains(power)
    }

    pub fn lowest_perimetr_hits(&self) -> Option<&StructureRampart> {
        let perimeter = self.state.plan.as_ref()
            .map(|plan| plan.perimeter())
            .unwrap_or_default();

        self.base.ramparts.iter()
            .filter(|r| perimeter.contains(&r.pos().xy()))
            .sorted_by_key(|r| r.hits())
            .next()
    }

    pub fn empty_sender(&self) -> Option<&StructureLink> {
        self.base.links.sender()
            .filter(|link| link.store().get_free_capacity(Some(ResourceType::Energy)) > 0)
    }

    pub fn full_receiver(&self) -> Option<&StructureLink> {
        self.base.links.receiver()
            .filter(|link| link.store().get_used_capacity(Some(ResourceType::Energy)) > 0)
    }

    pub fn requests(&self) -> impl Iterator<Item = &Request> {
        self.state.requests.iter()
    }

    pub fn resolve_request(&mut self, request: Request, doer: String) {
        let removed = self.state.requests.remove(&request);
        debug!("{} resolved request: {} {:?}", doer, removed, request);
    }

    pub fn replace_request(&mut self, request: Request) {
        self.state.requests.replace(request);
    }

    pub fn add_request(&mut self, request: Request) {
        self.state.requests.insert(request);
    }

    pub fn take_request(&mut self, request: &Request) -> Option<Request> {
        self.state.requests.take(request)
    }

    pub fn add_to_spawn(&mut self, role: Role, times: usize) {
        self.state.add_to_spawn(role, times);
    }

    pub fn storage(&self) -> Option<&StructureStorage> {
        self.base.storage()
    }

    pub fn controller(&self) -> &StructureController {
        &self.base.controller
    }

    pub fn pc_workplace(&self) -> Option<Position> {
        self.state.plan.as_ref()
            .and_then(|plan| plan.pc_workplace())
            .map(|xy| Position::new(xy.x, xy.y, self.name()))
    }

    pub fn power_spawn(&self) -> Option<&StructurePowerSpawn> {
        self.base.power_spawn.as_ref()
    }

    pub fn factory(&self) -> Option<&StructureFactory> {
        self.base.factory()
    }

    pub fn terminal(&self) -> Option<&StructureTerminal> {
        self.base.terminal()
    }

    pub fn production_labs(&self) -> (&[StructureLab], &[StructureLab]) {
        (self.base.labs.inputs(), self.base.labs.outputs())
    }

    pub fn lab_for_boost(&self, resources: &[ResourceType; 2]) -> Option<ObjectId<StructureLab>> {
        resources.iter()
            .find_map(|res| self.base.labs.boost_lab(res))
    }

    pub fn mineral(&self) -> &Mineral {
        &self.base.mineral
    }

    pub fn all_minerals(&self) -> impl Iterator<Item = &Mineral> {
        self.base.all_minerals()
    }

    pub fn all_sources(&self) -> impl Iterator<Item = &Source> {
        self.base.all_sources()
    }

    pub fn find_source_near(&self, pos: &Position) -> Option<ObjectId<Source>> {
        self.base.sources.iter()
            .chain(self.base.farms.iter().flat_map(|farm| farm.sources.iter()))
            .find_map(|source| {
                if pos.is_near_to(source.pos()) {
                    Some(source.id())
                } else {
                    None
                }
            })
    }

    pub fn find_container_in_range(&self, pos: Position, range: u32) -> Option<(RawObjectId, Position)> {
        //todo 1 trait for containers and links?
        self.base.links.ctrl()
            .map(|link| (link.raw_id(), link.pos()))
            .or_else(|| self.base.containers.iter()
                .find_map(|c| {
                    if pos.get_range_to(c.pos()) <= range {
                        Some((c.raw_id(), c.pos()))
                    } else {
                        None
                    }
                }))
    }
    
    pub fn get_available_boost(&self, creep: &Creep, all_boosts: HashMap<Part, [ResourceType; 2]>)
        -> Option<(ObjectId<StructureLab>, Part)>
    {
        creep.body().iter()
            .filter(|bodypart| bodypart.boost().is_none())
            .map(|bodypart| bodypart.part())
            .unique()
            .flat_map(|part| all_boosts.get(&part).map(|resoucres| (part, resoucres)))
            .find_map(|resources_for_part| self.lab_for_boost(resources_for_part.1)
                .map(|id| (id, resources_for_part.0)))
    }

    pub fn unload<T>(&self, obj: &T, allowed: &[ResourceType]) -> Option<RoomEvent>
        where T: HasStore + HasId
    {
        self.base.unload(obj, allowed)
    }

    pub fn load_lab(&self, lab: &StructureLab, component: (ResourceType, u32)) -> Option<RoomEvent> {
        self.base.load_lab(lab, component)
    }

    pub fn supply_resources(&self, to: RawObjectId, resource: ResourceType, amount: u32) -> Option<RoomEvent> {
        self.base.supply_resources(to, resource, amount)
    }

    pub fn tower_without_effect(&self) -> Option<&StructureTower> {
        self.base.towers.iter()
            .find(|tower| !tower.effects().into_iter()
                .any(|effect:Effect| {
                    match effect.effect() {
                        EffectType::PowerEffect(p) => matches!(p, PowerType::OperateTower),
                        _ => false
                    }
                }))
    }

    pub fn spawn_without_effect(&self) -> Option<&StructureSpawn> {
        self.base.spawns.iter()
            .find(|spawn| !spawn.effects().into_iter()
                .any(|effect:Effect| {
                    match effect.effect() {
                        EffectType::PowerEffect(p) => matches!(p, PowerType::OperateSpawn),
                        _ => false
                    }
                }))
    }

    pub fn factory_without_effect(&self) -> Option<&StructureFactory> {
        self.base.factory.as_ref()
            .filter(|factory| !factory.effects().into_iter()
                .any(|effect:Effect| {
                    match effect.effect() {
                        EffectType::PowerEffect(p) => matches!(p, PowerType::OperateFactory),
                        _ => false
                    }
                }))
    }

    pub fn full_storage_without_effect(&self) -> Option<&StructureStorage> {
        self.base.storage().filter(|storage| storage.effects().is_empty() && storage.store().get_used_capacity(None) > 990000)
    }

    pub fn mineral_without_effect(&self) -> bool {
        self.base.mineral.ticks_to_regeneration().is_none() && !self.base.mineral.effects().into_iter()
            .any(|effect:Effect| {
                match effect.effect() {
                    EffectType::PowerEffect(p) => matches!(p, PowerType::RegenMineral),
                    _ => false
                }
            })
    }

    pub fn source_without_effect(&self) -> Option<&Source> {
        //todo check remote rooms sources for powers without hardcoded ids
        self.base.sources.iter()
            .find(|source| !source.effects().into_iter()
                .any(|effect:Effect| {
                    match effect.effect() {
                        EffectType::PowerEffect(p) => {
                            matches!(p, PowerType::RegenSource if { effect.ticks_remaining() > 30 })
                        },
                        _ => false
                    }
                }))
    }
}