use std::collections::HashMap;

use log::info;
use screeps::{
    ConstructionSite, Creep, Event, HasHits, HasId, HasPosition, INVADER_USERNAME, MaybeHasId,
    Mineral, Nuke, Part, PowerCreep, RESOURCES_ALL, RawObjectId, Resource, ResourceType, Room,
    RoomName, SharedCreepProperties, Source, StructureContainer, StructureController,
    StructureExtension, StructureFactory, StructureNuker, StructureObject, StructureObserver,
    StructurePowerSpawn, StructureRoad, StructureSpawn, StructureStorage, StructureTerminal,
    StructureTower, StructureWall, Tombstone, find, game,
};
use smallvec::SmallVec;

use crate::commons::has_part;
use crate::resources::Resources;
use crate::rooms::{
    RoomEvent, RoomState,
    state::{
        BoostReason,
        constructions::RoomPlan,
        requests::{
            BodyPart, BuildData, CarryData, CreepHostile, PickupData, RepairData, Request,
            RequestKind, WithdrawData, assignment::Assignment,
        },
    },
    wrappers::{
        Fillable,
        claimed::structures::{labs::Labs, links::Links, ramparts::Ramparts},
    },
};
use crate::units::{
    creeps::CreepMemory,
    roles::{Role, combat::guard::Guard},
};
use crate::utils::constants::{
    MAX_WALL_HITS, MIN_PERIMETR_HITS, MY_ROOMS_PICKUP_RESOURCE_THRESHOLD,
};

mod structures;

//todo implement prelude.rs
pub(crate) struct Claimed {
    pub(crate) room: Room,
    pub(crate) controller: StructureController,
    // pub(crate) farms: Vec<Farm>, //todo move to shelter
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
    pub(crate) events: Vec<Event>,
}

impl Claimed {
    pub(crate) fn new(room: Room, state: &RoomState) -> Self {
        let controller = room.controller().expect("expect controller in my Base");
        let mineral = room.find(find::MINERALS, None).remove(0);
        let sources = room.find(find::SOURCES, None);
        let hostiles = room.find(find::HOSTILE_CREEPS, None);
        let my_creeps = room.find(find::MY_CREEPS, None);
        let my_power_creeps = room.find(find::MY_POWER_CREEPS, None);
        let landing_nukes = room.find(find::NUKES, None);
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
                StructureObject::StructureTower(tower) => towers.push(tower),
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

        let amounts = if game::time().is_multiple_of(100) {
            RESOURCES_ALL
                .iter()
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
                    for lab in &labs {
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
            my_pcreeps: my_power_creeps,
            nukes: landing_nukes,
            tombs,
            cs,
            dropped,
            events,
            resources: Resources::new(amounts),
        }
    }

    pub fn supply_resources(
        &self,
        to: RawObjectId,
        resource: ResourceType,
        amount: u32,
    ) -> Option<RoomEvent> {
        self.storage()
            .and_then(|storage| {
                let storage_capacity = storage.store().get_used_capacity(Some(resource));
                if storage_capacity < amount
                    || (ResourceType::Energy == resource && storage_capacity < 25_000)
                {
                    None
                } else {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Carry(CarryData::new(storage.raw_id(), to, resource, amount)),
                        Assignment::Single(None),
                    )))
                }
            })
            .or_else(|| {
                self.factory().filter(|f| f.raw_id() != to).and_then(|f| {
                    let f_capacity = f.store().get_used_capacity(Some(resource));
                    if f_capacity < amount
                        || ResourceType::Energy == resource && f_capacity < 10_000
                    {
                        None
                    } else {
                        Some(RoomEvent::Request(Request::new(
                            RequestKind::Carry(CarryData::new(f.raw_id(), to, resource, amount)),
                            Assignment::Single(None),
                        )))
                    }
                })
            })
    }

    pub fn get_name(&self) -> RoomName {
        self.room.name()
    }

    pub const fn storage(&self) -> Option<&StructureStorage> {
        self.storage.as_ref()
    }

    pub const fn terminal(&self) -> Option<&StructureTerminal> {
        self.terminal.as_ref()
    }

    pub const fn factory(&self) -> Option<&StructureFactory> {
        self.factory.as_ref()
    }

    pub fn energy_available(&self) -> u32 {
        self.room.energy_available()
    }

    pub fn energy_capacity_available(&self) -> u32 {
        self.room.energy_capacity_available()
    }

    //todo the Box requires cloning, consider to avoid cloning by adding new
    // lifetime to Task -> CreepMemory -> Unit
    pub(crate) fn closest_empty_structure(
        &self,
        to: &dyn HasPosition,
    ) -> Option<Box<dyn Fillable>> {
        self.extensions
            .iter()
            .filter(|e| e.store().get_free_capacity(Some(ResourceType::Energy)) > 0)
            .cloned()
            .map(|e| Box::new(e) as Box<dyn Fillable>)
            .chain(
                self.towers
                    .iter()
                    .filter(|t| t.store().get_free_capacity(Some(ResourceType::Energy)) > 0)
                    .cloned()
                    .map(|t| Box::new(t) as Box<dyn Fillable>),
            )
            .chain(
                self.spawns
                    .iter()
                    .filter(|s| s.store().get_free_capacity(Some(ResourceType::Energy)) > 0)
                    .cloned()
                    .map(|s| Box::new(s) as Box<dyn Fillable>),
            )
            .min_by_key(|f| to.pos().get_range_to(f.position()))
    }

    pub fn build_requests(&self) -> Vec<Request> {
        self.cs
            .iter()
            .filter(|cs| cs.my())
            .map(move |cs| {
                Request::new(
                    RequestKind::Build(BuildData::new(cs.try_id(), cs.pos())),
                    Assignment::Single(None),
                )
            })
            .collect()
    }

    pub fn pickup_requests(&self) -> Vec<Request> {
        self.dropped
            .iter()
            .filter_map(move |resource| {
                if (resource.resource_type() == ResourceType::Energy
                    && resource.amount() > MY_ROOMS_PICKUP_RESOURCE_THRESHOLD)
                    || (resource.resource_type() != ResourceType::Energy && resource.amount() >= 50)
                {
                    Some(Request::new(
                        RequestKind::Pickup(PickupData::new(resource.id())),
                        Assignment::Single(None),
                    ))
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
            .map(move |tomb| {
                Request::new(
                    RequestKind::Withdraw(WithdrawData::new(
                        tomb.id().into(),
                        tomb.pos(),
                        tomb.store().store_types().into_iter().map(|res| (res, None)).collect(),
                    )),
                    Assignment::Single(None),
                )
            })
            .collect()
    }

    pub fn repair_roads(&self, plan: Option<&RoomPlan>) -> impl Iterator<Item = Request> {
        let mut requests = Vec::new();
        if let Some(plan) = plan {
            let planned_roads = plan.roads();
            for road in &self.roads {
                if road.hits() * 4 < road.hits_max() * 2 && planned_roads.contains(&road.pos().xy())
                {
                    let attempts = if road.hits_max() == 5_000 { 2 } else { 5 };
                    requests.push(Request::new(
                        RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                            road.id().into_type(),
                            road.pos(),
                            attempts,
                            road.hits(),
                        )),
                        Assignment::Single(None),
                    ));
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
                        wall.hits(),
                    )),
                    Assignment::Single(None),
                ));
            }
        }
        requests.into_iter()
    }

    //todo logic similar to logic for towers
    pub fn security_check(
        &self,
        room_memory: &RoomState,
        creeps: &HashMap<String, CreepMemory>,
    ) -> impl Iterator<Item = RoomEvent> {
        self.invasion_check(room_memory, creeps)
            .into_iter()
            .chain(self.perimetr_check())
            .chain((!self.nukes.is_empty()).then(|| RoomEvent::NukeFalling))
    }

    pub fn invasion_check(
        &self,
        room_memory: &RoomState,
        creeps: &HashMap<String, CreepMemory>,
    ) -> SmallVec<[RoomEvent; 4]> {
        let mut events: SmallVec<[RoomEvent; 4]> = SmallVec::new();
        match self.controller.level() {
            //todo create different claimed room types
            ..=3 => {
                //any parts
                if self.hostiles.iter().any(|hostile| {
                    hostile.owner().username() != INVADER_USERNAME
                        && has_part(
                            &[Part::Attack, Part::RangedAttack, Part::Claim],
                            hostile,
                            false,
                        )
                }) {
                    events
                        .push(RoomEvent::Intrusion(Some(format!("{} Invasion!", self.get_name()))));
                    let _ = self.controller.activate_safe_mode();
                }
            }
            4..=6 => {
                if !find_player_boosted_creeps(&self.hostiles).is_empty() && self.towers.is_empty()
                {
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
                        5.. if alive_number < 3 => 3 - alive_number,
                        _ => 0,
                    };
                    info!(
                        "{} alive guards: {} boosted_enemies: {}, to spawn: {}",
                        self.get_name(),
                        alive_number,
                        boosted_enemies.len(),
                        to_spawn
                    );
                    events.push(RoomEvent::Spawn(guard, to_spawn));
                    events.push(RoomEvent::Intrusion(Some(format!(
                        "Boosted player invasion in room {}!!",
                        self.get_name()
                    ))));
                } else if room_memory.intrusion && room_memory.last_intrusion + 100 < game::time() {
                    events.push(RoomEvent::Intrusion(None));
                }
            }
        }
        events
    }

    fn perimetr_check(&self) -> Option<RoomEvent> {
        (self.controller.level() > 5
            && self.ramparts.perimeter().any(|rampart| rampart.hits() < MIN_PERIMETR_HITS))
        .then(|| {
            RoomEvent::ActivateSafeMode("Perimeter out of order! Enabling safe mode!".to_string())
        })
    }
}

fn find_player_boosted_creeps(enemies: &[Creep]) -> Vec<CreepHostile> {
    enemies
        .iter()
        .filter(|creep| {
            creep.owner().username() != INVADER_USERNAME
                && creep.body().iter().any(|body_part| body_part.boost().is_some())
        })
        .map(|hostile| CreepHostile {
            name: hostile.name(),
            owner: hostile.owner().username(),
            ticks_to_live: hostile.ticks_to_live(),
            parts: hostile
                .body()
                .iter()
                .map(|bodypart| BodyPart {
                    boosted: bodypart.boost().is_some(),
                    part: bodypart.part(),
                    hits: bodypart.hits(),
                })
                .collect(),
        })
        .collect()
}
