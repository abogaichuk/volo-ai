use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;

use itertools::Itertools;
use log::warn;
use screeps::{
    ConstructionSite, Creep, Deposit, HasPosition, Mineral, ObjectId, Part, Position, RawObjectId,
    Resource, ResourceType, RoomCoordinate, RoomName, SOURCE_KEEPER_USERNAME, Source, Structure,
    StructureController, StructureInvaderCore, StructureLab, StructurePowerBank, find,
};

use super::roles::Role;
use super::with_parts;
use crate::commons::{closest_attacker, find_closest_exit, find_hostiles, say_message};
use crate::movement::walker::Walker;
use crate::rooms::state::requests::{Request, RequestKind};
use crate::rooms::wrappers::Fillable;
use crate::units::{MovementGoal, has_part};

/// Distance from center to edge
const PROBE_DISTANCE: u32 = 24;
const FLEE_RANGE: u32 = 5;

mod boost;
mod build;
mod combat;
mod deposit;
mod dismantle;
mod escape;
mod harvest;
mod logistics;
mod powerbank;
mod repair;
mod reservation;
mod upgrade;

//todo implement prelude
pub enum Task {
    DefendHome,
    MoveMe(RoomName, Walker), /* task to get inside room only! in the target room -> abort
                               * followed by new task */
    Portal(Position),
    Flee(u32),
    Escape(Position),
    Provoke(u32, u32),
    Hide(Position, u32),
    Build(Option<ObjectId<ConstructionSite>>, Position),
    Repair(ObjectId<Structure>, Position, u8),
    HarvestEnergyForever(Position, ObjectId<Source>),
    HarvestMineral(Position, ObjectId<Mineral>),
    Harvest(Position, ObjectId<Source>),
    HarvestAndUpgrade(Position, ObjectId<Source>, ObjectId<StructureController>),
    PowerbankAttack(Position, ObjectId<StructurePowerBank>, HashSet<String>),
    PowerbankHeal(Position, ObjectId<StructurePowerBank>, HashSet<String>),
    PowerbankCarry(Position, ObjectId<StructurePowerBank>),
    DepositHarvest(Position, ObjectId<Deposit>),
    DepositCarry(Position),
    Upgrade(ObjectId<StructureController>, Option<RawObjectId>),
    TakeResource(ObjectId<Resource>),
    FillStructure(Box<dyn Fillable>),
    Dismantle(ObjectId<Structure>, Position),
    PullTo(String, Position),
    TakeFromStructure(Position, RawObjectId, ResourceType, Option<u32>),
    DeliverToStructure(Position, RawObjectId, ResourceType, Option<u32>),
    Withdraw(Position, RawObjectId, Vec<(ResourceType, Option<u32>)>),
    LongRangeWithdraw(Position, RawObjectId, ResourceType, u32),
    GenerateSafeMode(Position, ObjectId<StructureController>, RawObjectId),
    Carry(RawObjectId, RawObjectId, ResourceType, u32, Option<Box<Task>>),
    Boost(ObjectId<StructureLab>, Option<u32>),
    Book(ObjectId<StructureController>, Position),
    Claim(ObjectId<StructureController>, Position),
    Crash(ObjectId<StructureInvaderCore>, Position),
    HealAll,
    Oversee(RoomName, Option<(Position, u32)>),
    Protect(RoomName, Option<Position>),
    Defend(RoomName),
    Idle(u32),
    Speak,
}

impl Default for Task {
    fn default() -> Task {
        Task::Idle(1)
    }
}

impl Task {
    pub fn run_task(self, creep: &Creep, role: &Role) -> TaskResult {
        let room = creep.room().expect("expect creep is in a room!");
        let hostiles: Vec<Creep> = find_hostiles(&room, Vec::new()).collect();

        match self {
            Task::Portal(pos) => {
                //todo implement jump into portal logic
                let goal = Walker::Exploring(false).walk(pos, 0, creep, role, Vec::new());
                TaskResult::StillWorking(Task::Portal(pos), Some(goal))
            }
            Task::Speak => {
                say_message(creep);
                TaskResult::StillWorking(Task::Speak, None)
            }
            Task::Idle(tick) => {
                if tick < 1 {
                    TaskResult::Abort
                } else {
                    let _ = creep.say("🚬", true);
                    TaskResult::StillWorking(Task::Idle(tick - 1), None)
                }
            }
            Task::MoveMe(room_name, walker) => {
                if creep.pos().room_name() == room_name {
                    let _ = creep.say("🗙", true);
                    TaskResult::Abort
                } else {
                    //todo bug here! huge cpu usage when overseer healing
                    let _ = creep.say("🚶🏻", true);
                    let position = unsafe {
                        Position::new(
                            RoomCoordinate::unchecked_new(25),
                            RoomCoordinate::unchecked_new(25),
                            room_name,
                        )
                    };
                    let attackers = with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]);

                    let goal = walker.walk(position, PROBE_DISTANCE, creep, role, attackers);
                    TaskResult::StillWorking(Task::MoveMe(room_name, walker), Some(goal))
                }
            }
            Task::Provoke(ticks, range) => {
                let attackers: Vec<Creep> = hostiles
                    .into_iter()
                    .filter(|hostile| has_part(&[Part::RangedAttack, Part::Attack], hostile, true))
                    .sorted_by_key(|hostile| creep.pos().get_range_to(hostile.pos()))
                    .collect();

                if let Some(closest_hostile) = attackers.first() {
                    let actual_range = creep.pos().get_range_to(closest_hostile.pos());
                    match actual_range.cmp(&range) {
                        Ordering::Less => {
                            let goal = Walker::Flee.walk(
                                closest_hostile.pos(),
                                10,
                                creep,
                                role,
                                attackers,
                            );
                            TaskResult::StillWorking(Task::Provoke(5, 10), Some(goal))
                        }
                        Ordering::Greater => {
                            let goal = Walker::Exploring(false).walk(
                                closest_hostile.pos(),
                                range,
                                creep,
                                role,
                                attackers,
                            );
                            TaskResult::StillWorking(Task::Provoke(5, range), Some(goal))
                        }
                        Ordering::Equal => match range {
                            5 => {
                                say_message(creep);
                                TaskResult::StillWorking(Task::Provoke(ticks, range), None)
                            }
                            ..5 => {
                                let goal = Walker::Flee.walk(
                                    closest_hostile.pos(),
                                    10,
                                    creep,
                                    role,
                                    attackers,
                                );
                                TaskResult::StillWorking(Task::Provoke(ticks, range), Some(goal))
                            }
                            _ => {
                                if ticks == 0 {
                                    TaskResult::StillWorking(Task::Provoke(5, range - 1), None)
                                } else {
                                    TaskResult::StillWorking(Task::Provoke(ticks - 1, range), None)
                                }
                            }
                        },
                    }
                } else {
                    let _ = creep.say("🚬", true);
                    TaskResult::Completed
                    // TaskResult::StillWorking(Task::Provoke(ticks, range),
                    // None)
                }
            }
            Task::Flee(range) => {
                //todo if time to escape
                let attackers = with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]);
                if let Some(closest_hostile) = closest_attacker(creep, attackers.iter()) {
                    if creep.pos().get_range_to(closest_hostile.pos()) < 6 {
                        let _ = creep.say("👎", true);

                        if closest_hostile.owner().username() != SOURCE_KEEPER_USERNAME
                            && let Some(exit) = find_closest_exit(creep, None)
                            && creep.pos().get_range_to(exit)
                                < creep.pos().get_range_to(closest_hostile.pos())
                        {
                            //todo stuck when all cells danger, then vec::new to avoid setting
                            // danger cells
                            let goal =
                                Walker::Exploring(false).walk(exit, 0, creep, role, Vec::new());
                            TaskResult::StillWorking(Task::Escape(exit), Some(goal))
                        } else {
                            //todo stuck when all cells danger, then vec::new to avoid setting
                            // danger cells
                            let goal = Walker::Flee.walk(
                                closest_hostile.pos(),
                                range,
                                creep,
                                role,
                                Vec::new(),
                            );
                            TaskResult::StillWorking(Task::Flee(range), Some(goal))
                        }
                    } else {
                        TaskResult::Abort
                    }
                } else {
                    TaskResult::Abort
                }
            }
            Task::Escape(position) => escape::escape(
                position,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::Hide(position, timeout) => escape::hide(
                position,
                timeout,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::Boost(id, parts_number) => boost::boost(id, parts_number, creep, role),
            Task::Harvest(workplace, id) => harvest::harvest_until_full(
                workplace,
                id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::HarvestAndUpgrade(workplace, id, ctrl_id) => harvest::harvest_and_upgrade(
                workplace,
                id,
                ctrl_id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::HarvestEnergyForever(workplace, id) => harvest::harvest_energy_forever(
                workplace,
                id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::HarvestMineral(workplace, id) => harvest::harvest_minerals(
                workplace,
                id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::PowerbankHeal(pos, id, members) => powerbank::pb_heal(
                pos,
                id,
                members,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack]),
            ),
            Task::PowerbankAttack(pos, id, members) => powerbank::pb_attack(
                pos,
                id,
                members,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack]),
            ),
            Task::PowerbankCarry(pos, id) => powerbank::pb_carry(
                pos,
                id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack]),
            ),
            Task::DepositHarvest(pos, id) => deposit::deposit_mine(
                pos,
                id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::DepositCarry(pos) => deposit::deposit_carry(
                pos,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::Build(id, pos) => build::build(id, pos, creep, role, hostiles),
            Task::Repair(id, pos, times) => repair::repair(id, pos, times, creep, role, hostiles),
            Task::Upgrade(id, container_id) => upgrade::upgrade(
                id,
                container_id,
                creep,
                role,
                with_parts(hostiles, vec![Part::Attack, Part::RangedAttack]),
            ),
            Task::Book(id, position) => reservation::book(
                id,
                position,
                creep,
                role,
                with_parts(hostiles, vec![Part::Attack, Part::RangedAttack]),
            ),
            Task::Claim(id, position) => reservation::claim(
                id,
                position,
                creep,
                role,
                with_parts(hostiles, vec![Part::Attack, Part::RangedAttack]),
            ),
            Task::PullTo(creep_name, destination) => logistics::pull_to(
                creep_name,
                destination,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::Dismantle(id, workplace) => dismantle::dismantle(id, workplace, creep, role),
            Task::TakeResource(id) => logistics::take_resource(
                id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::TakeFromStructure(pos, id, resources, room_reqested) => {
                logistics::take_from_structure(
                    pos,
                    id,
                    resources,
                    room_reqested,
                    creep,
                    role,
                    with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
                )
            }
            Task::DeliverToStructure(pos, id, resource, amount) => logistics::deliver_to_structure(
                pos,
                id,
                resource,
                amount,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::FillStructure(structure) => logistics::fill_structure(
                structure,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::Withdraw(pos, id, resources) => logistics::withdraw(
                pos,
                id,
                resources,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::GenerateSafeMode(pos, id, storage_id) => logistics::generate_safe_mode(
                pos,
                id,
                storage_id,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::LongRangeWithdraw(pos, id, resource, amount) => logistics::long_range_withdraw(
                pos,
                id,
                resource,
                amount,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::Carry(from, to, resource, amount, _) => logistics::carry(
                from,
                to,
                resource,
                amount,
                creep,
                role,
                with_parts(hostiles, vec![Part::RangedAttack, Part::Attack]),
            ),
            Task::DefendHome => combat::defend_home(creep, role, hostiles),
            Task::Crash(id, pos) => combat::crash(id, pos, creep, role, hostiles),
            Task::HealAll => combat::heal_all(creep, role, hostiles),
            Task::Defend(room_name) => {
                combat::defend(room_name, creep, role, hostiles)
            }
            Task::Oversee(room_name, target) => {
                combat::oversee(room_name, target, creep, role, hostiles)
            }
            //todo target position need here to keep track on the hostile movement
            Task::Protect(target_room, target_pos) => {
                let structures = room.find(find::STRUCTURES, None);
                combat::protect(target_room, target_pos, creep, role, hostiles, structures)
            }
        }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Task::DefendHome => write!(f, "Task::DefendHome"),
            Task::Speak => write!(f, "Task::Speak"),
            Task::Idle(ticks) => write!(f, "Task::Idle[{ticks}]"),
            Task::Provoke(ticks, range) => write!(f, "Task::Provoke[{ticks}, {range}]"),
            Task::Portal(pos) => write!(f, "Task::Portal[{pos}]"),
            Task::MoveMe(room_name, walk_type) => {
                write!(f, "Task::MoveMe[{room_name}, {walk_type:?}]")
            }
            Task::Flee(range) => write!(f, "Task::Flee[{range}]"),
            Task::Escape(pos) => write!(f, "Task::Escape[{pos}]"),
            Task::Hide(pos, timeout) => write!(f, "Task::Hide[{pos}, {timeout}]"),
            Task::Upgrade(id, container_id) => {
                write!(f, "Task::Upgrade[{id}, {container_id:?}]")
            }
            Task::TakeResource(id) => write!(f, "Task::TakeResource[{id}]"),
            Task::HealAll => write!(f, "Task::HealAll"),
            Task::Oversee(room_name, target) => {
                write!(f, "Task::Oversee[{room_name}, {target:?}]")
            }
            Task::Boost(id, parts) => write!(f, "Task::Boost[{id}, {parts:?}]"),
            Task::Crash(id, pos) => write!(f, "Task::Crash[{id}, {pos:?}]"),
            Task::Defend(room_name) => {
                write!(f, "Task::Defend[{room_name}]")
            }
            Task::Protect(pos, target_pos) => write!(f, "Task::Protect[{pos}, {target_pos:?}]"),
            Task::Build(id, pos) => write!(f, "Task::Build[{id:?}, {pos}]"),
            Task::Repair(id, pos, times) => write!(f, "Task::Repair[{id}, {pos}, {times}]"),
            Task::HarvestEnergyForever(id, pos) => {
                write!(f, "Task::HarvestEnergyForever[{id}, {pos}]")
            }
            Task::HarvestMineral(id, pos) => write!(f, "Task::HarvestMineral[{id}, {pos}]"),
            Task::Harvest(pos, id) => write!(f, "Task::Harvest[{id}, {pos}]"),
            Task::HarvestAndUpgrade(pos, id, ctrl_id) => {
                write!(f, "Task::HarvestAndUpgrade[{pos}, {id}, {ctrl_id}]")
            }
            Task::PowerbankAttack(pos, id, members) => {
                write!(f, "Task::PowerbankAttack[{id}, {pos}, {members:?}]")
            }
            Task::PowerbankHeal(pos, id, members) => {
                write!(f, "Task::PowerbankHeal[{id}, {pos}, {members:?}]")
            }
            Task::PowerbankCarry(pos, id) => write!(f, "Task::PowerbankCarry[{id}, {pos}]"),
            Task::DepositHarvest(pos, id) => write!(f, "Task::DepositHarvest[{id}, {pos}]"),
            Task::DepositCarry(pos) => write!(f, "Task::DepositCarry[{pos}]"),
            Task::Dismantle(id, pos) => write!(f, "Task::Dismantle[{id}, {pos}]"),
            Task::PullTo(name, pos) => write!(f, "Task::PullTo[{name}, {pos}]"),
            Task::Book(id, pos) => write!(f, "Task::Book[{id}, {pos}]"),
            Task::Claim(id, pos) => write!(f, "Task::Claim[{id}, {pos}]"),
            Task::TakeFromStructure(pos, id, resource, amount) => {
                write!(f, "Task::TakeFromStructure[{pos}, {id}, {resource}, {amount:?}]")
            }
            Task::Withdraw(pos, id, resources) => {
                write!(f, "Task::Withdraw[{pos}, {id}, {resources:?}]")
            }
            Task::LongRangeWithdraw(pos, id, resource, amount) => {
                write!(f, "Task::LongRangeWithdraw[{pos}, {id}, {resource}, {amount}]")
            }
            Task::GenerateSafeMode(pos, id, storage_id) => {
                write!(f, "Task::GenerateSafeMode[{pos}, {id}, {storage_id}]")
            }
            Task::FillStructure(structure) => {
                write!(f, "Task::FillStructure[{}]", structure.position())
            }
            // Task::FillStructures(empty_structures) => write!(f, "Task::FillStructures[{:?}]",
            // empty_structures),
            Task::DeliverToStructure(pos, id, resource, amount) => {
                write!(f, "Task::DeliverToStructure[{pos}, {id}, {resource}, {amount:?}]")
            }
            Task::Carry(from, to, resource, amount, _) => {
                write!(f, "Task::Carry[{from}, {to}, {resource}, {amount}]")
            }
        }
    }
}

impl From<(Request, Role)> for Task {
    fn from((req, role): (Request, Role)) -> Self {
        match (req.kind, role) {
            (RequestKind::Deposit(d), Role::DepositHauler(_)) => Task::DepositCarry(d.pos),
            (RequestKind::Deposit(d), Role::DepositMiner(_)) => Task::DepositHarvest(d.pos, d.id),
            (RequestKind::Powerbank(d), Role::PBAttacker(pba)) => pba
                .squad_id
                .as_ref()
                .and_then(|squad_id| req.assignment.squads_members(squad_id))
                .map(|members| Task::PowerbankAttack(d.pos, d.id, members))
                .unwrap_or_default(),
            (RequestKind::Powerbank(d), Role::PBHealer(pbh)) => pbh
                .squad_id
                .as_ref()
                .and_then(|squad_id| req.assignment.squads_members(squad_id))
                .map(|members| Task::PowerbankHeal(d.pos, d.id, members))
                .unwrap_or_default(),
            (RequestKind::Powerbank(d), Role::PBCarrier(_)) => Task::PowerbankCarry(d.pos, d.id),
            (kind, _) => <Task as From<RequestKind>>::from(kind),
        }
    }
}

impl From<RequestKind> for Task {
    fn from(request_kind: RequestKind) -> Self {
        match request_kind {
            RequestKind::Claim(r) => Task::Claim(r.id, r.pos),
            RequestKind::Book(r) => Task::Book(r.id, r.pos),
            RequestKind::Build(r) => Task::Build(r.id, r.pos),
            RequestKind::Repair(r) => Task::Repair(r.id, r.pos, r.attempts_max),
            RequestKind::Defend(r) => Task::Defend(r.room_name),
            RequestKind::Carry(r) => Task::Carry(r.from, r.to, r.resource, r.amount, None),
            RequestKind::Withdraw(r) => Task::Withdraw(r.pos, r.id, r.resources),
            RequestKind::LongRangeWithdraw(r) => {
                Task::LongRangeWithdraw(r.pos, r.id, r.resource, r.amount)
            }
            RequestKind::SafeMode(r) => Task::GenerateSafeMode(r.pos, r.id, r.storage_id),
            RequestKind::Pickup(r) => Task::TakeResource(r.id),
            RequestKind::Pull(r) => Task::PullTo(r.creep_name, r.destination),
            RequestKind::Dismantle(r) => Task::Dismantle(r.id, r.workplace),
            RequestKind::Crash(r) => Task::Crash(r.id, r.pos),
            _ => {
                warn!("weird room request: {:?}", request_kind);
                Task::Idle(10)
            }
        }
    }
}

#[derive(Debug, Default)]
pub enum TaskResult {
    RunAnother(Task),
    StillWorking(Task, Option<MovementGoal>),
    ResolveRequest(Task), //graceful suicide
    UpdateRequest(Task),        /* update room request (in the middle of doing something,
                                 * partially carried resource or repair structure) */
    AddNewRequest(Task, Task, Option<MovementGoal>),
    Suicide, //finish a task with suicide
    Completed,
    #[default]
    Abort, //find another task on the same tick
}
