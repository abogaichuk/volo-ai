use log::warn;
use screeps::{
    Creep, Deposit, HasId, HasPosition, ObjectId, Part, Position, ResourceType,
    SharedCreepProperties, Tombstone, find, game,
};

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::tasks::FLEE_RANGE;
use crate::units::{Task, TaskResult, has_part};
use crate::utils::commons::{closest_attacker, get_place_to_store};
use crate::utils::constants::CLOSE_RANGE_ACTION;

pub fn deposit_mine(
    pos: Position,
    id: ObjectId<Deposit>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if creep.pos().room_name() != pos.room_name() {
        TaskResult::RunAnother(Task::MoveMe(pos.room_name(), Walker::Exploring(false)))
    } else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
        .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 4)
    {
        let goal = Walker::Flee.walk(in_range_attacker.pos(), 5, creep, role, enemies);
        TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
    } else if creep.pos().is_near_to(pos) {
        if let Some(deposit) = id.resolve() {
            if deposit.cooldown() == 0
                && creep.store().get_free_capacity(Some(deposit.deposit_type())) > 0
            {
                let _ = creep.harvest(&deposit);
            } else if creep.store().get_used_capacity(Some(deposit.deposit_type())) > 0
                && let Some(mule) = get_near_by_hauler(creep)
            {
                let _ = creep.transfer(&mule, deposit.deposit_type(), None);
            } else {
                let _ = creep.say("🚬", true);
            }
            TaskResult::StillWorking(Task::DepositHarvest(pos, id), None)
        } else {
            TaskResult::ResolveRequest(Task::DepositHarvest(pos, id))
        }
    } else {
        let goal = Walker::Exploring(true).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::DepositHarvest(pos, id), Some(goal))
    }
}

pub fn deposit_carry(pos: Position, creep: &Creep, role: &Role, enemies: Vec<Creep>) -> TaskResult {
    //todo use shelter instead
    let home_room =
        role.get_home().and_then(|home| game::rooms().get(*home)).expect("expect role has a home!");

    if is_full(creep)
        || (creep.ticks_to_live().is_some_and(|ticks| ticks < 200)
            && creep.store().get_used_capacity(None) > 0)
    {
        if let Some(container) = get_place_to_store(&home_room) {
            let resources = creep.store().store_types();
            if let Some(resource_type) = resources.first() {
                let goal = Walker::Exploring(false).walk(
                    container.pos(),
                    CLOSE_RANGE_ACTION,
                    creep,
                    role,
                    enemies,
                );
                TaskResult::StillWorking(
                    Task::DeliverToStructure(
                        container.pos(),
                        container.as_structure().raw_id(),
                        *resource_type,
                        None,
                    ),
                    Some(goal),
                )
            } else {
                warn!("{} {} injured!", creep.name(), home_room.name());
                TaskResult::Abort
            }
        } else {
            warn!("{} {} there is no place to store!", creep.name(), home_room.name());
            TaskResult::Abort
        }
    } else if let Some((tomb, resource)) = loaded_tomb(creep) {
        let goal =
            Walker::Exploring(false).walk(tomb.pos(), CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(
            Task::TakeFromStructure(tomb.pos(), tomb.raw_id(), resource, None),
            Some(goal),
        )
    } else if creep.pos().room_name() != pos.room_name() {
        let goal = Walker::Exploring(false).walk(pos, 4, creep, role, enemies);
        TaskResult::StillWorking(Task::DepositCarry(pos), Some(goal))
    } else if let Some(miner) = loaded_miner(creep) {
        let goal =
            Walker::Exploring(true).walk(miner.pos(), CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::DepositCarry(pos), Some(goal))
    } else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
        .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 4)
    {
        let goal = Walker::Flee.walk(in_range_attacker.pos(), 5, creep, role, enemies);
        TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
    } else if !creep.pos().in_range_to(pos, 4) {
        let goal = Walker::Exploring(true).walk(pos, 4, creep, role, enemies);
        TaskResult::StillWorking(Task::DepositCarry(pos), Some(goal))
    } else {
        TaskResult::StillWorking(Task::Idle(1), None)
    }
}

fn loaded_miner(creep: &Creep) -> Option<Creep> {
    let room = creep.room().expect("expect creep is in a room!");
    room.find(find::MY_CREEPS, None).into_iter().find(|miner| {
        has_part(&[Part::Work], miner, false) && miner.store().get_used_capacity(None) > 0
    })
}

fn loaded_tomb(creep: &Creep) -> Option<(Tombstone, ResourceType)> {
    let room = creep.room().expect("expect creep is in a room!");
    room.find(find::TOMBSTONES, None).into_iter().find_map(|tomb| {
        tomb.store()
            .store_types()
            .iter()
            .find(|res| **res != ResourceType::Energy)
            .map(|res| (tomb, *res))
    })
}

fn is_full(creep: &Creep) -> bool {
    creep.store().get_used_capacity(None)
        >= creep.store().get_capacity(None) - creep.store().get_capacity(None) / 25 // 4%
}

fn get_near_by_hauler(to: &Creep) -> Option<Creep> {
    let room = to.room().expect("expect creep is in a room!");
    room.find(find::MY_CREEPS, None).into_iter().find(|creep| {
        creep.pos().is_near_to(to.pos())
            && has_part(&[Part::Carry], creep, true)
            && !has_part(&[Part::Work], creep, false)
    })
}
