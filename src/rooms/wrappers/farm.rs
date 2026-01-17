use log::debug;
use screeps::{
    Creep, EffectType, Event, HasHits, HasId, HasPosition, MaybeHasId, Mineral, Part, Room,
    RoomName, RoomObjectProperties, SOURCE_KEEPER_USERNAME, SharedCreepProperties, Source,
    StructureContainer, StructureController, StructureInvaderCore, StructureKeeperLair,
    StructureObject, StructureRoad, find, game,
};

use crate::commons::{capture_room_numbers, get_room_regex};
use crate::rooms::state::FarmInfo;
use crate::rooms::state::constructions::RoomPlan;
use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::{
    BodyPart, BookData, BuildData, CrashData, CreepHostile, PickupData, RepairData, Request,
    RequestKind, WithdrawData,
};
use crate::rooms::{RoomEvent, is_extractor, missed_buildings};
use crate::units::roles::Role;
use crate::units::roles::miners::mineral_miner::MineralMiner;
use crate::utils::constants::FARM_ROOMS_PICKUP_RESOURCE_THRESHOLD;

pub struct Farm {
    pub(crate) room: Room,
    pub(crate) memory: FarmInfo,
    pub(crate) hostiles: Vec<Creep>,
    pub(crate) mineral: Option<Mineral>,
    pub(crate) sources: Vec<Source>,
    pub(crate) containers: Vec<StructureContainer>,
    pub(crate) roads: Vec<StructureRoad>,
    pub(crate) icore: Option<StructureInvaderCore>,
    pub(crate) keepers: Vec<StructureKeeperLair>,
    pub(crate) events: Vec<Event>,
    kind: FarmKind,
}

impl Farm {
    pub fn new(room: Room, memory: FarmInfo) -> Self {
        let mut containers = Vec::new();
        let mut roads = Vec::new();
        let mineral = room.find(find::MINERALS, None).into_iter().find(is_extractor);
        let sources = room.find(find::SOURCES, None);
        let hostiles = room.find(find::HOSTILE_CREEPS, None);
        let events = if hostiles.is_empty() { Vec::new() } else { room.get_event_log() };

        for structure in room.find(find::STRUCTURES, None) {
            match structure {
                StructureObject::StructureContainer(c) => containers.push(c),
                StructureObject::StructureRoad(r) => roads.push(r),
                _ => {}
            }
        }

        let mut icore = None;
        let mut keepers = Vec::new();
        for structure in room.find(find::HOSTILE_STRUCTURES, None) {
            match structure {
                StructureObject::StructureInvaderCore(ic) => icore = Some(ic),
                StructureObject::StructureKeeperLair(k) => keepers.push(k),
                _ => {}
            }
        }

        let kind = FarmKind::from(room.name());

        Self {
            room,
            memory,
            hostiles,
            mineral,
            sources,
            containers,
            roads,
            icore,
            keepers,
            events,
            kind,
        }
    }

    pub const fn room(&self) -> &Room {
        &self.room
    }

    pub fn get_name(&self) -> RoomName {
        self.room.name()
    }

    pub fn is_active(&self) -> bool {
        self.memory.is_active()
    }

    pub fn run_farm(&self) -> Vec<RoomEvent> {
        match self.kind {
            //if reservable or central and is active -> farm this room
            FarmKind::Reservable | FarmKind::Central if self.memory.is_active() => {
                self.create_cs(self.memory.plan());
                self.get_farm_requests(self.memory.plan())
            }
            FarmKind::SourceKeeperRoom => {
                let ic_timeout = self
                    .icore
                    .as_ref()
                    .and_then(|ic| {
                        ic.effects().iter().find_map(|effect| {
                            match effect.effect() {
                                //add 50 ticks to make sure a request with collapse timer has been
                                // created
                                EffectType::NaturalEffect(_) => Some(effect.ticks_remaining() + 50),
                                EffectType::PowerEffect(_) => None,
                            }
                        })
                    })
                    .unwrap_or_default();

                if ic_timeout > 0 {
                    // invander core is in the room! insert to avoid_rooms
                    vec![
                        RoomEvent::Avoid(self.get_name(), game::time() + ic_timeout),
                        RoomEvent::UpdateFarmStatus(self.get_name(), false),
                    ]
                } else if !self.memory.is_active() {
                    // no invander core in the room -> enable farming
                    vec![RoomEvent::UpdateFarmStatus(self.get_name(), true)]
                } else {
                    //active farm and no invander cores
                    self.create_cs(self.memory.plan());
                    self.get_farm_requests(self.memory.plan())
                }
            }
            _ => {
                //farm is temporarly forbiden, do nothing
                vec![]
            }
        }
    }

    fn create_cs(&self, plan: Option<&RoomPlan>) {
        if let Some(plan) = plan
            && game::time().is_multiple_of(100)
        {
            missed_buildings(self.get_name(), plan).for_each(|(xy, str_type)| {
                let _ = self.room.create_construction_site(xy.x.u8(), xy.y.u8(), str_type, None);
            });
        }
    }

    fn get_farm_requests(&self, plan: Option<&RoomPlan>) -> Vec<RoomEvent> {
        let mut events = Vec::new();

        if let Some(event) = self.defend_request() {
            events.push(event);
        } else if let Some(black_list_event) = self.check_log() {
            events.push(black_list_event);
        } else {
            if let Some(plan) = plan {
                events.extend(self.run_containers(plan));
                events.extend(self.repair_roads(plan));
            }
            events.extend(self.build_requests());
            events.extend(self.pickup_requests());
            events.extend(self.tomb_requests());

            if let Some(controller) = self.room().controller() {
                events.extend(self.crash_request());
                events.extend(self.reserve_room(&controller));
            } else if game::time().is_multiple_of(100) {
                events.extend(self.mineral_event());
            }
        }
        events
    }

    const fn check_log(&self) -> Option<RoomEvent> {
        //todo implement check logs logic
        // self.events.iter()
        //     .find(|event| )
        None
    }

    fn mineral_event(&self) -> Option<RoomEvent> {
        if let Some(mineral) = &self.mineral
            && mineral.mineral_amount() > 0
            && is_extractor(mineral)
        {
            self.containers.iter().find_map(|container| {
                if container.pos().is_near_to(mineral.pos()) {
                    let mineral_miner =
                        Role::MineralMiner(MineralMiner::new(Some(container.pos()), None));
                    Some(RoomEvent::MayBeSpawn(mineral_miner))
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    fn pickup_requests(&self) -> Vec<RoomEvent> {
        self.room()
            .find(find::DROPPED_RESOURCES, None)
            .iter()
            .filter_map(move |resource| {
                if resource.amount() > FARM_ROOMS_PICKUP_RESOURCE_THRESHOLD {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Pickup(PickupData::new(resource.id())),
                        Assignment::Single(None),
                    )))
                    // Some(RoomEvent::Request(Request::Pickup(PickupRequest::new(resource.id()))))
                } else {
                    None
                }
            })
            .collect()
    }

    fn tomb_requests(&self) -> Vec<RoomEvent> {
        self.room()
            .find(find::TOMBSTONES, None)
            .iter()
            .filter(|tomb| tomb.store().get_used_capacity(None) > 1000)
            .map(|tomb| {
                let resources =
                    tomb.store().store_types().into_iter().map(|res| (res, None)).collect();
                RoomEvent::Request(Request::new(
                    RequestKind::Withdraw(WithdrawData::new(
                        tomb.id().into(),
                        tomb.pos(),
                        resources,
                    )),
                    Assignment::Single(None),
                ))
            })
            .collect()
    }

    fn run_containers(&self, plan: &RoomPlan) -> Vec<RoomEvent> {
        let planned_containers = plan.containers();
        self.containers
            .iter()
            .filter_map(|container| {
                if container.store().get_used_capacity(None) >= 1250 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Withdraw(WithdrawData::new(
                            container.id().into(),
                            container.pos(),
                            container
                                .store()
                                .store_types()
                                .into_iter()
                                .map(|res| (res, None))
                                .collect(),
                        )),
                        Assignment::Single(None),
                    )))
                } else if container.hits() * 4 < container.hits_max() * 2
                    && planned_containers.contains(&container.pos().xy())
                {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                            container.id().into_type(),
                            container.pos(),
                            10,
                            container.hits(),
                        )),
                        Assignment::Single(None),
                    )))
                } else {
                    None
                }
            })
            .collect()
    }

    fn repair_roads(&self, plan: &RoomPlan) -> Vec<RoomEvent> {
        let planned_roads = plan.roads();
        self.roads
            .iter()
            .filter_map(|road| {
                if road.hits() < road.hits_max() / 2 && planned_roads.contains(&road.pos().xy()) {
                    let attempts = if road.hits_max() == 5000 { 2 } else { 5 };
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                            road.id().into_type(),
                            road.pos(),
                            attempts,
                            road.hits(),
                        )),
                        Assignment::Single(None),
                    )))
                } else {
                    None
                }
            })
            .collect()
    }

    fn build_requests(&self) -> Vec<RoomEvent> {
        self.room()
            .find(find::CONSTRUCTION_SITES, None)
            .iter()
            .filter(|cs| cs.my())
            .map(|cs| {
                RoomEvent::Request(Request::new(
                    RequestKind::Build(BuildData::new(cs.try_id(), cs.pos())),
                    Assignment::Single(None),
                ))
            })
            .collect()
    }

    fn crash_request(&self) -> Option<RoomEvent> {
        self.icore.as_ref().map(|ic| {
            RoomEvent::Request(Request::new(
                RequestKind::Crash(CrashData::new(ic.id(), ic.pos())),
                Assignment::Single(None),
            ))
        })
    }

    fn reserve_room(&self, controller: &StructureController) -> Option<RoomEvent> {
        controller.reservation().is_none_or(|reservation| reservation.ticks_to_end() < 1000).then(
            || {
                RoomEvent::Request(Request::new(
                    RequestKind::Book(BookData::new(controller.id(), controller.pos())),
                    Assignment::Single(None),
                ))
            },
        )
    }

    fn defend_request(&self) -> Option<RoomEvent> {
        let parts = [Part::Attack, Part::RangedAttack, Part::Claim, Part::Carry, Part::Work];
        let enemies: Vec<CreepHostile> = self
            .hostiles
            .iter()
            .filter(|hostile| {
                hostile
                    .body()
                    .iter()
                    .map(screeps::BodyPart::part)
                    .any(|part| parts.is_empty() || parts.contains(&part))
            })
            .filter(|creep| creep.owner().username() != SOURCE_KEEPER_USERNAME)
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
            .collect();

        if !enemies.is_empty() {
            if game::time().is_multiple_of(50) {
                debug!("enemies {} in room: {}", enemies.len(), self.get_name());
            }
            return Some(RoomEvent::Defend(self.get_name()));
        }
        None
    }

    pub fn get_hostiles(&self) -> &[Creep] {
        &self.hostiles
    }
}

pub enum FarmKind {
    Reservable,
    SourceKeeperRoom,
    Central,
}

impl From<RoomName> for FarmKind {
    fn from(name: RoomName) -> Self {
        let re = get_room_regex();
        if let Some((f_num, s_num)) = capture_room_numbers(&re, name) {
            let (f_reminder, s_reminder) = (f_num % 10, s_num % 10);

            if is_central(f_reminder, s_reminder) {
                FarmKind::Central
            } else if is_skr(f_reminder, s_reminder) {
                FarmKind::SourceKeeperRoom
            } else {
                FarmKind::Reservable
            }
        } else {
            FarmKind::Reservable
        }
    }
}

fn is_central(f_mod: u32, s_mod: u32) -> bool {
    f_mod == 5 && s_mod == 5
}

fn is_skr(f_mod: u32, s_mod: u32) -> bool {
    (4..=6).contains(&f_mod) && (4..=6).contains(&s_mod)
}
