use log::*;
use serde::{Serialize, Deserialize};
use std::{collections::{HashMap, HashSet}, fmt::{Display, Formatter}, hash::{Hash, Hasher}};
use screeps::Part;
use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    rooms::{RoomEvent, shelter::Shelter},
    units::{creeps::CreepMemory, tasks::Task}
};
use self::{
    meta::{Meta, Status}, assignment::Assignment,
    data::{
        destroy::destroy_handler, protect::protect_handler, defend::defend_handler, transfer::transfer_handler,
        factory::factory_handler, lab::lab_handler, power_bank::powerbank_handler, deposit::deposit_handler,
        caravan::caravan_handler, build::build_handler, repair::repair_handler, claim::claim_handler,
        book::book_handler, dismantle::dismantle_handler, crash::crash_handler, safe_mode::sm_handler,
        pull::pull_handler, pickup::pickup_handler, withdraw::withdraw_handler, carry::carry_handler,
        lrw::lrw_handler
    }
};

pub use data::{
    LabData, TransferData, FactoryData,
    DestroyData, DefendData, ProtectData, CrashData,
    CaravanData, PowerbankData, DepositData,
    ClaimData, BookData, BuildData, RepairData, DismantleData, PullData,
    PickupData, WithdrawData, LRWData, CarryData, SMData};

pub mod meta;
pub mod assignment;
mod data;

//todo defend request for all defenders
//todo repair perimeter for all house keepers
//defend home for all guards
//carry something massive for all carriers
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub assignment: Assignment,
    pub kind: RequestKind,
    #[serde(flatten, default)]
    pub meta: Meta
}

impl Request {
    pub fn new(kind: RequestKind, assignment: Assignment) -> Self {
        Self { assignment, kind, meta: Meta::default() }
    }

    pub fn with_meta(kind: RequestKind, assignment: Assignment, meta: Meta) -> Self {
        Self { assignment, kind, meta }
    }

    pub fn status(&self) -> &Status { &self.meta.status }
    pub fn assigned_to(&self, name: &str) -> bool { self.assignment.has_member(name) }
    pub fn created_at(&self) -> u32 { self.meta.created_at }

    pub fn handle(&mut self, home: &Shelter, creeps: &HashMap<String, CreepMemory>) -> SmallVec<[RoomEvent; 3]> {
        let (meta, assignment, kind) = (
            &mut self.meta,
            &mut self.assignment,
            &mut self.kind,
        );

        match kind {
            RequestKind::Destroy(_) => destroy_handler(),
            RequestKind::Protect(d) => protect_handler(d, meta, home.name()),
            RequestKind::Defend(d) => defend_handler(d, meta, assignment, home, creeps),
            RequestKind::Transfer(d) => transfer_handler(d, meta, home),
            RequestKind::Factory(d) => factory_handler(d, meta, home),
            RequestKind::Lab(d) => lab_handler(d, meta, &home.base),
            RequestKind::Powerbank(d) => powerbank_handler(d, meta, assignment, home),
            RequestKind::Deposit(d) => deposit_handler(d, meta, assignment, home.name()),
            RequestKind::Caravan(d) => caravan_handler(d, meta, home.name()),
            RequestKind::Build(_) => build_handler(meta, assignment),
            RequestKind::Repair(_) => repair_handler(meta, assignment),
            RequestKind::Claim(_) => claim_handler(meta, assignment, home.name()),
            RequestKind::Book(b) => book_handler(b, meta, assignment, home.name()),
            RequestKind::Dismantle(d) => dismantle_handler(d, meta, assignment, home.name()),
            RequestKind::Crash(_) => crash_handler(meta, assignment, home, creeps),
            RequestKind::SafeMode(_) => sm_handler(meta, assignment, home.name()),
            RequestKind::Pull(_) => pull_handler(),
            RequestKind::Pickup(_) => pickup_handler(meta),
            RequestKind::Withdraw(_) => withdraw_handler(meta, assignment),
            RequestKind::Carry(_) => carry_handler(meta, assignment),
            RequestKind::LongRangeWithdraw(_) => lrw_handler(meta, assignment, home.name())
        }
    }

    // pub fn handle(self, home: &Shelter, creeps: &HashMap<String, Memory>) -> SmallVec<[RoomEvent; 3]> {
    //     let (mut meta, mut assignment, mut kind) = (self.meta, self.assignment, self.kind);

    //     let mut events = match &mut kind {
    //         RequestKind::Destroy(_) => destroy_handler(),
    //         RequestKind::Protect(d) => protect_handler(d, &mut meta, home.name()),
    //         RequestKind::Defend(d) => defend_handler(d, &mut meta, &mut assignment, home, creeps),
    //         RequestKind::Transfer(d) => transfer_handler(d, &mut meta, home),
    //         RequestKind::Factory(d) => factory_handler(d, &mut meta, home),
    //         RequestKind::Lab(d) => lab_handler(d, &mut meta, &home.base),
    //         RequestKind::Powerbank(d) => powerbank_handler(d, &mut meta, &mut assignment, home),
    //         RequestKind::Deposit(d) => deposit_handler(d, &mut meta, &mut assignment, home.name()),
    //         RequestKind::Caravan(d) => caravan_handler(d, &mut meta, home.name()),
    //         RequestKind::Build(_) => build_handler(&mut meta, &mut assignment),
    //         RequestKind::Repair(_) => repair_handler(&mut meta, &mut assignment),
    //         RequestKind::Claim(_) => claim_handler(&mut meta, &mut assignment, home.name()),
    //         RequestKind::Book(b) => book_handler(b, &mut meta, &mut assignment, home.name()),
    //         RequestKind::Dismantle(d) => dismantle_handler(d, &mut meta, &mut assignment, home.name()),
    //         RequestKind::Crash(_) => crash_handler(&mut meta, &mut assignment, home, creeps),
    //         RequestKind::SafeMode(_) => sm_handler(&mut meta, &mut assignment, home.name()),
    //         RequestKind::Pull(_) => pull_handler(),
    //         RequestKind::Pickup(_) => pickup_handler(&mut meta),
    //         RequestKind::Withdraw(_) => withdraw_handler(&mut meta, &mut assignment),
    //         RequestKind::Carry(_) => carry_handler(&mut meta, &mut assignment),
    //         RequestKind::LongRangeWithdraw(_) => lrw_handler(&mut meta, &mut assignment, home.name())
    //     };

    //     if !matches!(meta.status, Status::Aborted | Status::Resolved) {
    //         events.extend_one(RoomEvent::Request(Request::with_meta(kind, assignment, meta)));
    //     }
    //     events
    // }

    pub fn join(
        &mut self,
        doer: Option<String>,
        squad_id: Option<&str>)
    {
        match self.assignment.try_join(doer, squad_id) {
            Ok(()) => {
                if matches!(self.meta.status, Status::Created | Status::Spawning) {
                    self.meta.update(Status::InProgress);
                }
            },
            Err(err) => error!("{}", err),
        };
    }
}

impl Hash for Request {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.kind {
            RequestKind::Pickup(d) => d.id.hash(state),
            RequestKind::Repair(d) => d.id.hash(state),
            RequestKind::Build(d) => d.id.hash(state),
            RequestKind::Claim(d) => d.id.hash(state),
            RequestKind::Book(d) => d.id.hash(state),
            RequestKind::Defend(d) => d.room_name.hash(state),
            RequestKind::Protect(d) => d.room_name.hash(state),
            RequestKind::Powerbank(d) => d.id.hash(state),
            RequestKind::Destroy(d) => d.target.hash(state),
            RequestKind::Deposit(d) => d.id.hash(state),
            RequestKind::Caravan(d) => d.ambush_room.hash(state),
            // RoomRequest::SCORE(score) => score.target.id.hash(state),
            RequestKind::Dismantle(d) => {
                d.id.hash(state);
                d.workplace.hash(state);
            },
            RequestKind::Crash(d) => d.id.hash(state),
            RequestKind::Pull(d) => d.creep_name.hash(state),
            RequestKind::Factory(d) => d.resource.hash(state),
            RequestKind::Lab(d) => d.resource.hash(state),
            RequestKind::Withdraw(d) => d.id.hash(state),
            RequestKind::LongRangeWithdraw(d) => d.id.hash(state),
            RequestKind::SafeMode(d) => d.id.hash(state),
            RequestKind::Transfer(d) => {
                d.resource.hash(state);
                d.destination.hash(state);
            },
            RequestKind::Carry(d)  => {
                d.from.hash(state);
                d.to.hash(state);
                d.resource.hash(state);
            }
        }
    }
}

impl Eq for Request {}
impl PartialEq for Request {
    fn eq(&self, other: &Request) -> bool {
        match &self.kind {
            RequestKind::Withdraw(d) => {
                match &other.kind {
                    RequestKind::Withdraw(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::LongRangeWithdraw(d) => {
                match &other.kind {
                    RequestKind::LongRangeWithdraw(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::SafeMode(d) => {
                match &other.kind {
                    RequestKind::SafeMode(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Pickup(d) => {
                match &other.kind {
                    RequestKind::Pickup(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Carry(d)  => {
                match &other.kind {
                    RequestKind::Carry(o) => d.from == o.from && d.to == o.to && d.resource == o.resource,
                    _ => false
                }
            },
            RequestKind::Repair(d)  => {
                match &other.kind {
                    RequestKind::Repair(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Build(d)  => {
                match &other.kind {
                    RequestKind::Build(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Claim(d) => {
                match &other.kind {
                    RequestKind::Claim(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Book(d) => {
                match &other.kind {
                    RequestKind::Book(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Defend(d) => {
                match &other.kind {
                    RequestKind::Defend(o) => d.room_name == o.room_name,
                    _ => false
                }
            },
            RequestKind::Protect(d) => {
                match &other.kind {
                    RequestKind::Protect(o) => d.room_name == o.room_name,
                    _ => false
                }
            },
            RequestKind::Powerbank(d)  => {
                match &other.kind {
                    RequestKind::Powerbank(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Destroy(d)  => {
                match &other.kind {
                    RequestKind::Destroy(o) => d.target == o.target,
                    _ => false
                }
            },
            RequestKind::Deposit(d)  => {
                match &other.kind {
                    RequestKind::Deposit(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Caravan(d)  => {
                match &other.kind {
                    RequestKind::Caravan(o) => d.ambush_room == o.ambush_room,
                    _ => false
                }
            },
            // RoomRequest::SCORE(request)  => {
            //     ScoreRequest::try_from(other.to_owned()).ok()
            //         .is_some_and(|another|
            //             request.target.id == another.target.id)
            // },
            RequestKind::Dismantle(d)  => {
                match &other.kind {
                    RequestKind::Dismantle(o) => d.id == o.id && d.workplace == o.workplace,
                    _ => false
                }
            },
            RequestKind::Crash(d)  => {
                match &other.kind {
                    RequestKind::Crash(o) => d.id == o.id,
                    _ => false
                }
            },
            RequestKind::Pull(d)  => {
                match &other.kind {
                    RequestKind::Pull(o) => d.creep_name.as_str() == o.creep_name.as_str(),
                    _ => false
                }
            },
            RequestKind::Factory(d)  => {
                match &other.kind {
                    RequestKind::Factory(o) => d.resource == o.resource,
                    _ => false
                }
            },
            RequestKind::Lab(d)  => {
                match &other.kind {
                    RequestKind::Lab(o) => d.resource == o.resource,
                    _ => false
                }
            },
            RequestKind::Transfer(d)  => {
                match &other.kind {
                    RequestKind::Transfer(o) => d.destination == o.destination && d.resource == o.resource,
                    _ => false
                }
            },
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "data: {}, assign: {}, meta: {:?}", self.kind, self.assignment, self.meta)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum RequestKind {
    Pickup(PickupData),
    Withdraw(WithdrawData),
    Carry(CarryData),
    LongRangeWithdraw(LRWData),
    SafeMode(SMData),
    Caravan(CaravanData),
    Repair(RepairData),
    Dismantle(DismantleData),
    Build(BuildData),
    Claim(ClaimData),
    Book(BookData),
    Pull(PullData),
    Defend(DefendData),
    Protect(ProtectData), //protect a room from hostile existance, 
    Destroy(DestroyData), //make target position walkable
    Crash(CrashData),
    Powerbank(PowerbankData),
    Deposit(DepositData),
    Factory(FactoryData),
    Lab(LabData),
    Transfer(TransferData),
}

impl Display for RequestKind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RequestKind::Pickup(d)          => write!(f, "Pickup({:?})", d),
            RequestKind::Withdraw(d)      => write!(f, "Withdraw({:?})", d),
            RequestKind::Carry(d)            => write!(f, "Carry({:?})", d),
            RequestKind::LongRangeWithdraw(d)  => write!(f, "LongRangeWithdraw({:?})", d),
            RequestKind::SafeMode(d)            => write!(f, "SafeMode({:?})", d),
            RequestKind::Caravan(d)        => write!(f, "Caravan({:?})", d),
            RequestKind::Repair(d)          => write!(f, "Repair({:?})", d),
            RequestKind::Dismantle(d)    => write!(f, "Dismantle({:?})", d),
            RequestKind::Build(d)            => write!(f, "Build({:?})", d),
            RequestKind::Claim(d)            => write!(f, "Claim({:?})", d),
            RequestKind::Book(d)              => write!(f, "Book({:?})", d),
            RequestKind::Pull(d)              => write!(f, "Pull({:?})", d),
            RequestKind::Defend(d)          => write!(f, "Defend({:?})", d),
            RequestKind::Protect(d)        => write!(f, "Protect({:?})", d), 
            RequestKind::Destroy(d)        => write!(f, "Destroy({:?})", d),
            RequestKind::Crash(d)            => write!(f, "Crash({:?})", d),
            RequestKind::Powerbank(d)    => write!(f, "Powerbank({:?})", d),
            RequestKind::Deposit(d)        => write!(f, "Deposit({:?})", d),
            RequestKind::Factory(d)        => write!(f, "Factory({:?})", d),
            RequestKind::Lab(d)                => write!(f, "Lab({:?})", d),
            RequestKind::Transfer(d)      => write!(f, "Transfer({:?})", d),
        }
    }
}

#[derive(Error, Debug)]
pub enum RequestError {
    // #[error("invalid assignment: {0} can't be assigned to {1}")]
    // InvalidAssignment(String, Assignment),
    #[error("invalid assignment: {0}")]
    InvalidAssignment(String),
    #[error("{0}: assignment busy {1}")]
    AssignmentBusy(String, Assignment),
    #[error("{0}: no squad_id provided")]
    EmptySquadId(String),
    #[error("invalid squad_id: {0}")]
    InvalidSquadId(String)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CreepHostile {
    pub name: String,
    pub owner: String,
    pub parts: Vec<BodyPart>,
    pub ticks_to_live: Option<u32>
}

impl CreepHostile {
    pub fn new(name: String, owner: String, parts: Vec<BodyPart>, ticks_to_live: Option<u32>) -> Self {
        Self { name, owner, parts, ticks_to_live }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BodyPart {
    pub boosted: bool,
    pub part: Part,
    pub hits: u32
}

impl TryFrom<Task> for Request {
    type Error = ();

    fn try_from(task: Task) -> Result<Self, Self::Error> {
        match task {
            Task::Book(id, pos) =>
                Ok(Request::new(RequestKind::Book(BookData::new(id, pos)), Assignment::Single(None))),
            Task::Claim(id, pos) =>
                Ok(Request::new(RequestKind::Claim(ClaimData::new(id, pos)), Assignment::Single(None))),
            Task::Repair(id, pos, times) =>
                Ok(Request::new(RequestKind::Repair(RepairData::new(id, pos, times)), Assignment::Single(None))),
            Task::Build(id, pos) =>
                Ok(Request::new(RequestKind::Build(BuildData::new(id, pos)), Assignment::Single(None))),
            Task::Carry(from, to, resource, amount, _) =>
                Ok(Request::new(RequestKind::Carry(CarryData::new(from, to, resource, amount)), Assignment::Single(None))),
            Task::Withdraw(pos, id, resources) =>
                Ok(Request::new(RequestKind::Withdraw(WithdrawData::new(id, pos, resources)), Assignment::Single(None))),
            Task::LongRangeWithdraw(pos, id, resource, amount) =>
                Ok(Request::new(RequestKind::LongRangeWithdraw(LRWData::new(id, pos, resource, amount)), Assignment::Single(None))),
            Task::GenerateSafeMode(pos, id, storage_id) =>
                Ok(Request::new(RequestKind::SafeMode(SMData::new(id, pos, storage_id)), Assignment::Single(None))),
            Task::TakeResource(id) =>
                Ok(Request::new(RequestKind::Pickup(PickupData::new(id)), Assignment::Single(None))),
            Task::PullTo(creep_name, destination) =>
                Ok(Request::new(RequestKind::Pull(PullData::new(creep_name, destination)), Assignment::Single(None))),
            Task::Dismantle(id, workplace) =>
                Ok(Request::new(RequestKind::Dismantle(DismantleData::new(id, workplace)), Assignment::Single(None))),
            Task::DepositHarvest(pos, id) =>
                Ok(Request::new(RequestKind::Deposit(DepositData::new(id, pos, 1)), Assignment::Squads(Vec::new()))),
            Task::PowerbankAttack(pos, id, _) =>
                Ok(Request::new(RequestKind::Powerbank(PowerbankData::new(id, pos, 1)), Assignment::Squads(Vec::new()))),
            Task::Crash(id, pos) =>
                Ok(Request::new(RequestKind::Crash(CrashData::new(id, pos)), Assignment::Single(None))),
            Task::Defend(room_name, room_requested) => {
                if room_requested {
                    Ok(Request::new(RequestKind::Defend(DefendData::new(room_name)), Assignment::Multi(HashSet::new())))
                } else {
                    Err(())
                }
            },
            _ => Err(())
        }
    }
}