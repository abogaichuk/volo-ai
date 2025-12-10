use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use log::{debug, error, info, warn};
use screeps::game::market::Order;
use screeps::game::{self};
use screeps::{
    Creep, Effect, EffectType, HasId, HasPosition, HasStore, Mineral, ObjectId, OrderType, Part,
    Position, PowerType, RawObjectId, ResourceType, Room, RoomName, RoomObjectProperties, Source,
    StructureController, StructureFactory, StructureLab, StructureLink, StructurePowerSpawn,
    StructureRampart, StructureSpawn, StructureStorage, StructureTerminal, StructureTower,
};

use crate::colony::ColonyEvent;
use crate::commons::find_roles;
use crate::rooms::{
    RoomEvent,
    state::{
        RoomState, TradeData,
        requests::{CreepHostile, DefendData, Request, RequestKind, assignment::Assignment},
    },
    wrappers::{Fillable, claimed::Claimed, farm::Farm},
};
use crate::statistics::RoomStats;
use crate::units::creeps::CreepMemory;
use crate::units::roles::Role;

pub struct Shelter<'s> {
    pub(crate) base: Claimed,
    pub(crate) state: &'s mut RoomState,
    pub(crate) white_list: &'s HashSet<String>,
}

impl<'s> Shelter<'s> {
    pub(crate) fn new(
        base_room: Room,
        farms: Vec<Farm>,
        state: &'s mut RoomState,
        white_list: &'s HashSet<String>,
    ) -> Self {
        Shelter { base: Claimed::new(base_room, farms, state), state, white_list }
    }

    pub fn run_shelter(
        &mut self,
        creeps: &mut HashMap<String, CreepMemory>,
        orders: &[Order],
    ) -> Vec<ColonyEvent> {
        // todo proportial perimetr security check
        let mut events = Vec::new();
        for mut request in self
            .state
            .requests
            .drain()
            .filter(|req| !req.meta.is_finished())
            .chain(self.base.run_ramparts())
            .chain(self.base.run_power())
            .chain(self.base.run_nuker())
            .chain(self.base.run_containers())
            .chain(self.base.repair_roads(self.state.plan.as_ref()))
            .chain(self.base.repair_walls())
            .chain(self.base.tomb_requests())
            .chain(self.base.pickup_requests())
            .chain(self.base.build_requests())
            .collect::<Vec<_>>()
        {
            events.extend(request.handle(self, creeps));
            self.add_request(request);
        }

        self.base.run_towers();
        self.base.run_links();
        self.base.run_observer();
        // lab, terminal and factory toogle request status to InProgress only
        // because only one request can be correctly handled at one tick
        events.extend(
            self.base
                .run_labs(self.state)
                .into_iter()
                .chain(self.base.run_factory(self.state))
                .chain(self.base.run_terminal(self.state, orders))
                .chain(self.base.security_check(self.state, creeps))
                .chain(self.base.run_spawns(self.state))
                .chain(self.base.time_based_events(self.state, creeps))
                .chain(self.base.farms.iter().flat_map(|farm| {
                    self.state
                        .farms
                        .get(&farm.get_name())
                        .map(|info| farm.run_farm(info))
                        .unwrap_or_default()
                        .into_iter()
                })),
        );

        let mut colony_events = Vec::new();
        for event in events {
            match event {
                RoomEvent::Request(request) => {
                    self.add_request(request);
                }
                RoomEvent::Defend(farm_name, hostiles) => {
                    let hostiles: Vec<CreepHostile> = hostiles
                        .into_iter()
                        .filter(|ch| !self.white_list.contains(&ch.name))
                        .collect();
                    self.add_request(Request::new(
                        RequestKind::Defend(DefendData::with_hostiles(farm_name, hostiles)),
                        Assignment::Multi(HashSet::new()),
                    ));
                }
                RoomEvent::ReplaceRequest(request) => {
                    self.replace_request(request);
                }
                RoomEvent::Spawned(name, role, index) => {
                    self.state.spawns.remove(index);
                    creeps.insert(name, CreepMemory::new(role));
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
                    for creep in creeps.iter_mut() {
                        if creep.1.role == role {
                            creep.1.respawned = true;
                        }
                    }
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
                }
                RoomEvent::StopFarm(room_name, with_room) => {
                    self.state.finish_farm(room_name, with_room);
                }
                RoomEvent::StartFarm(room_name, with_room) => {
                    self.state.begin_farm(room_name, with_room);
                }
                RoomEvent::Lack(res, amount) => {
                    colony_events.push(ColonyEvent::Lack(self.name(), res, amount));
                }
                RoomEvent::Excess(res, amount) => {
                    colony_events.push(ColonyEvent::Excess(self.name(), res, amount));
                }
                RoomEvent::Sell(order_id, resource, amount) => {
                    if let Some(mut trade) =
                        self.state.trades.take(&TradeData::new(OrderType::Sell, resource))
                    {
                        match game::market::deal(&order_id, amount, Some(self.name())) {
                            Ok(()) => {
                                trade.amount -= amount;
                                self.state.trades.insert(trade);
                                info!("{} sell: {}", self.name(), resource);
                            }
                            Err(err) => {
                                error!("sell trade error: {:?}", err);
                            }
                        }
                    } else {
                        error!(
                            "{} not found trade {:?} resource {}",
                            self.name(),
                            OrderType::Sell,
                            resource
                        );
                    }
                }
                RoomEvent::Buy(order_id, resource, amount) => {
                    if let Some(mut trade) =
                        self.state.trades.take(&TradeData::new(OrderType::Buy, resource))
                    {
                        match game::market::deal(&order_id, amount, Some(self.name())) {
                            Ok(()) => {
                                trade.amount -= amount;
                                self.state.trades.insert(trade);
                                info!("{} buy: {}", self.name(), resource);
                            }
                            Err(err) => {
                                error!("buy trade error: {:?}", err);
                            }
                        }
                    } else {
                        error!(
                            "{} not found trade {:?} resource {}",
                            self.name(),
                            OrderType::Sell,
                            resource
                        );
                    }
                }
                RoomEvent::Intrusion(message) => {
                    if let Some(message) = message {
                        self.state.intrusion = true;
                        self.state.last_intrusion = game::time();
                        colony_events.push(ColonyEvent::Notify(message, Some(30)));
                    } else {
                        self.state.intrusion = false;
                    }
                }
                RoomEvent::NukeFalling => {
                    let land_time = self.base.nukes.iter().map(screeps::Nuke::time_to_land).min();
                    let message = format!(
                        "Nuke is launched to room {}, splash in: {:?}",
                        self.name(),
                        land_time
                    );
                    colony_events.push(ColonyEvent::Notify(message, Some(30)));
                }
                RoomEvent::AddPlans(plans) => {
                    for (name, additional) in plans {
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
                    let _ = self.base.controller.activate_safe_mode();
                    warn!("{} activated safe mode!!", self.name());
                    colony_events.push(ColonyEvent::Notify(message, Some(30)));
                }
                RoomEvent::UpdateStatistic => {
                    let creeps_number = creeps
                        .iter()
                        .filter(|(_, memory)| {
                            memory.role.get_home().is_some_and(|home| *home == self.name())
                        })
                        .count();
                    let requests = self.state.requests.len();
                    let last_intrusion = self.state.last_intrusion;

                    colony_events.push(ColonyEvent::Stats(
                        self.name(),
                        RoomStats::new(&self.base, requests, last_intrusion, creeps_number),
                    ));
                } /* RoomEvent::Sos => {
                   *     warn!("room event sos is not implemented yet!");
                   *     //todo colony help me
                   * } */
            }
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
        self.base.get_farms().iter().map(super::wrappers::farm::Farm::get_name)
    }

    pub(crate) fn base(self) -> Claimed {
        self.base
    }

    pub const fn room(&self) -> &Room {
        &self.base.room
    }

    pub const fn invasion(&self) -> bool {
        self.state.intrusion
    }

    pub fn is_power_enabled(&self, power: PowerType) -> bool {
        self.state.powers.contains(&power)
    }

    pub fn closest_empty_structure(&self, to: &dyn HasPosition) -> Option<Box<dyn Fillable>> {
        self.base.closest_empty_structure(to)
    }

    pub fn lowest_perimetr_hits(&self) -> Option<&StructureRampart> {
        self.base.ramparts.perimeter().sorted_by_key(screeps::HasHits::hits).next()
    }

    pub fn empty_sender(&self) -> Option<&StructureLink> {
        self.base
            .links
            .sender()
            .filter(|link| link.store().get_free_capacity(Some(ResourceType::Energy)) > 0)
    }

    pub fn full_receiver(&self) -> Option<&StructureLink> {
        self.base
            .links
            .receiver()
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
        let replaced = self.state.requests.replace(request);
        debug!("replaced request: {:?}", replaced);
    }

    pub fn add_request(&mut self, request: Request) -> bool {
        self.state.requests.insert(request)
    }

    pub fn take_request(&mut self, request: &Request) -> Option<Request> {
        self.state.requests.take(request)
    }

    pub fn get_request(&mut self, request: &Request) -> Option<&Request> {
        self.state.requests.get(request)
    }

    pub fn add_to_spawn(&mut self, role: Role, times: usize) {
        self.state.add_to_spawn(role, times);
    }

    pub const fn storage(&self) -> Option<&StructureStorage> {
        self.base.storage()
    }

    pub const fn controller(&self) -> &StructureController {
        &self.base.controller
    }

    pub fn pc_workplace(&self) -> Option<Position> {
        self.state
            .plan
            .as_ref()
            .and_then(super::state::constructions::RoomPlan::pc_workplace)
            .map(|xy| Position::new(xy.x, xy.y, self.name()))
    }

    pub const fn power_spawn(&self) -> Option<&StructurePowerSpawn> {
        self.base.power_spawn.as_ref()
    }

    pub const fn factory(&self) -> Option<&StructureFactory> {
        self.base.factory()
    }

    pub const fn terminal(&self) -> Option<&StructureTerminal> {
        self.base.terminal()
    }

    pub fn production_labs(&self) -> (&[StructureLab], &[StructureLab]) {
        (self.base.labs.inputs(), self.base.labs.outputs())
    }

    pub fn lab_for_boost(&self, resources: [ResourceType; 2]) -> Option<ObjectId<StructureLab>> {
        resources.iter().find_map(|res| self.base.labs.boost_lab(*res))
    }

    pub const fn mineral(&self) -> &Mineral {
        &self.base.mineral
    }

    pub fn all_minerals(&self) -> impl Iterator<Item = &Mineral> {
        self.base.all_minerals()
    }

    pub fn all_sources(&self) -> impl Iterator<Item = &Source> {
        self.base.all_sources()
    }

    pub fn find_source_near(&self, pos: Position) -> Option<ObjectId<Source>> {
        self.base
            .sources
            .iter()
            .chain(self.base.farms.iter().flat_map(|farm| farm.sources.iter()))
            .find_map(|source| if pos.is_near_to(source.pos()) { Some(source.id()) } else { None })
    }

    pub fn find_container_in_range(
        &self,
        pos: Position,
        range: u32,
    ) -> Option<(RawObjectId, Position)> {
        //todo 1 trait for containers and links?
        self.base.links.ctrl().map(|link| (link.raw_id(), link.pos())).or_else(|| {
            self.base.containers.iter().find_map(|c| {
                if pos.get_range_to(c.pos()) <= range { Some((c.raw_id(), c.pos())) } else { None }
            })
        })
    }

    pub fn get_available_boost(
        &self,
        creep: &Creep,
        all_boosts: HashMap<Part, [ResourceType; 2]>,
    ) -> Option<(ObjectId<StructureLab>, Part)> {
        creep
            .body()
            .iter()
            .filter(|bodypart| bodypart.boost().is_none())
            .map(screeps::BodyPart::part)
            .unique()
            .filter_map(|part| all_boosts.get(&part).map(|resoucres| (part, resoucres)))
            .find_map(|resources_for_part| {
                self.lab_for_boost(*resources_for_part.1).map(|id| (id, resources_for_part.0))
            })
    }

    pub fn unload<T>(&self, obj: &T, allowed: &[ResourceType]) -> Option<RoomEvent>
    where
        T: HasStore + HasId,
    {
        self.base.unload(obj, allowed)
    }

    pub fn supply_resources(
        &self,
        to: RawObjectId,
        resource: ResourceType,
        amount: u32,
    ) -> Option<RoomEvent> {
        self.base.supply_resources(to, resource, amount)
    }

    pub fn tower_without_effect(&self) -> Option<&StructureTower> {
        self.base.towers.iter().find(|tower| {
            !tower.effects().into_iter().any(|effect: Effect| match effect.effect() {
                EffectType::PowerEffect(p) => matches!(p, PowerType::OperateTower),
                EffectType::NaturalEffect(_) => false,
            })
        })
    }

    pub fn spawn_without_effect(&self) -> Option<&StructureSpawn> {
        self.base.spawns.iter().find(|spawn| {
            !spawn.effects().into_iter().any(|effect: Effect| match effect.effect() {
                EffectType::PowerEffect(p) => matches!(p, PowerType::OperateSpawn),
                EffectType::NaturalEffect(_) => false,
            })
        })
    }

    pub fn factory_without_effect(&self) -> Option<&StructureFactory> {
        self.base.factory.as_ref().filter(|factory| {
            !factory.effects().into_iter().any(|effect: Effect| match effect.effect() {
                EffectType::PowerEffect(p) => matches!(p, PowerType::OperateFactory),
                EffectType::NaturalEffect(_) => false,
            })
        })
    }

    pub fn full_storage_without_effect(&self) -> Option<&StructureStorage> {
        self.base.storage().filter(|storage| {
            storage.effects().is_empty() && storage.store().get_used_capacity(None) > 990_000
        })
    }

    pub fn mineral_without_effect(&self) -> bool {
        self.base.mineral.ticks_to_regeneration().is_none()
            && !self.base.mineral.effects().into_iter().any(|effect: Effect| {
                match effect.effect() {
                    EffectType::PowerEffect(p) => matches!(p, PowerType::RegenMineral),
                    EffectType::NaturalEffect(_) => false,
                }
            })
    }

    pub fn source_without_effect(&self) -> Option<&Source> {
        //todo check remote rooms sources for powers without hardcoded ids
        self.base.sources.iter().find(|source| {
            !source.effects().into_iter().any(|effect: Effect| match effect.effect() {
                EffectType::PowerEffect(p) => {
                    matches!(p, PowerType::RegenSource if { effect.ticks_remaining() > 30 })
                }
                EffectType::NaturalEffect(_) => false,
            })
        })
    }
}
