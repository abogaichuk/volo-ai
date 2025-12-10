use std::collections::HashSet;

use log::warn;
use screeps::{
    Creep, HasId, HasPosition, ObjectId, Position, ResourceType, Room, Ruin, SharedCreepProperties,
    StructurePowerBank, find, game,
};

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::tasks::FLEE_RANGE;
use crate::units::tasks::logistics::{deliver_to_structure, take_from_structure, take_resource};
use crate::units::{Task, TaskResult};
use crate::utils::commons::{closest_attacker, find_dropped, get_place_to_store};
use crate::utils::constants::{CLOSE_RANGE_ACTION, LONG_RANGE_ACTION};

pub fn pb_attack(
    pos: Position,
    id: ObjectId<StructurePowerBank>,
    members: HashSet<String>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if let Some(healer) = members
        .iter()
        .find(|name| **name != creep.name())
        .and_then(|member| game::creeps().get(member.clone()))
        && (creep.pos().is_near_to(healer.pos()) || creep.pos().is_room_edge())
    {
        // healer is near attacker
        if creep.pos().room_name() != pos.room_name() {
            //if in another room -> go to powerbank room
            let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
            TaskResult::StillWorking(Task::PowerbankAttack(pos, id, members), Some(goal))
        }
        // else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
        //     .filter(|enemy| enemy.pos().get_range_to(creep.pos()) == 1)
        // {
        //     if BLACK_LIST.contains(&in_range_attacker.owner().username().as_str()) {

        //     } else if WHITE_LIST.contains(&in_range_attacker.owner().username().as_str()) {

        //     } else {
        //         let goal = Walker::Flee.walk(in_range_attacker.pos(), FLEE_RANGE, creep, role,
        // enemies);         TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
        //     }
        // }
        else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
            .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 4)
        {
            let goal = Walker::Flee.walk(in_range_attacker.pos(), FLEE_RANGE, creep, role, enemies);
            TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
        } else if closest_attacker(creep, enemies.iter())
            .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 7)
            .is_some()
        {
            //if enemies in range 7 -> stop attack powerbank to keep hit points full
            TaskResult::StillWorking(Task::PowerbankAttack(pos, id, members), None)
        } else if creep.pos().is_near_to(pos) {
            if let Some(powerbank) = id.resolve() {
                let _ = creep.attack(&powerbank);
                TaskResult::StillWorking(Task::PowerbankAttack(pos, id, members), None)
            } else {
                let _ = creep.suicide();
                TaskResult::StillWorking(Task::PowerbankAttack(pos, id, members), None)
            }
        } else {
            let goal = Walker::Exploring(false).walk(pos, 1, creep, role, enemies);
            TaskResult::StillWorking(Task::PowerbankAttack(pos, id, members), Some(goal))
        }
    } else {
        //wait for healer getting fresh members on the next tick
        TaskResult::Completed
        // TaskResult::StillWorking(Task::PowerbankAttack(pos, id, members),
        // None)
    }
}

pub fn pb_heal(
    pos: Position,
    id: ObjectId<StructurePowerBank>,
    members: HashSet<String>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if let Some(pb_attacker) = members
        .iter()
        .find(|name| **name != creep.name())
        .and_then(|member| game::creeps().get(member.clone()))
    {
        //if member found
        if pb_attacker.pos().is_near_to(pos) && creep.pos().is_near_to(pb_attacker.pos()) {
            if creep.hits() < creep.hits_max() {
                let _ = creep.heal(creep);
            } else {
                let _ = creep.heal(&pb_attacker);
            }
            TaskResult::StillWorking(Task::PowerbankHeal(pos, id, members), None)
        } else {
            let goal = Walker::Therapeutic.walk(pb_attacker.pos(), 0, creep, role, enemies);
            TaskResult::StillWorking(Task::PowerbankHeal(pos, id, members), Some(goal))
        }
    } else if creep.pos().is_near_to(pos) {
        //if no attacker and near powerbank -> means attacker commited suicide
        let _ = creep.suicide();
        TaskResult::Completed
    } else {
        //wait for attacker getting fresh members on the next tick
        let _ = creep.heal(creep);
        TaskResult::Completed
        // TaskResult::StillWorking(Task::PowerbankHeal(pos, id, members), None)
    }
}

pub fn pb_carry(
    pos: Position,
    id: ObjectId<StructurePowerBank>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    //todo use shelter instead
    let home_room =
        role.get_home().and_then(|home| game::rooms().get(*home)).expect("expect role has a home!");
    if creep.store().get_used_capacity(Some(ResourceType::Power)) > 0 {
        if let Some(storage) = get_place_to_store(&home_room) {
            match deliver_to_structure(
                storage.pos(),
                storage.as_structure().raw_id(),
                ResourceType::Power,
                None,
                creep,
                role,
                enemies,
            ) {
                TaskResult::StillWorking(_, movement_goal) => {
                    TaskResult::StillWorking(Task::PowerbankCarry(pos, id), movement_goal)
                }
                result => {
                    let _ = creep.suicide();
                    result
                }
            }
        } else {
            warn!("{} {} there is no place to store power!", creep.name(), home_room.name());
            TaskResult::Completed
        }
    } else if creep.pos().room_name() != pos.room_name() {
        //if in another room -> go to powerbank room
        let goal = Walker::Exploring(false).walk(pos, CLOSE_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::PowerbankCarry(pos, id), Some(goal))
    } else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
        .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 4)
    {
        let goal = Walker::Flee.walk(in_range_attacker.pos(), FLEE_RANGE, creep, role, enemies);
        TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
    } else if closest_attacker(creep, enemies.iter())
        .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 7)
        .is_some()
    {
        //if enemies in range 7 -> stop attack powerbank to keep hit points full
        TaskResult::StillWorking(Task::PowerbankCarry(pos, id), None)
    } else if creep.pos().in_range_to(pos, 3) {
        if id.resolve().is_none() {
            let room = creep.room().expect("expect creep is in a room!");
            if let Some(ruin) = find_ruin(&room) {
                take_from_structure(
                    ruin.pos(),
                    ruin.raw_id(),
                    ResourceType::Power,
                    None,
                    creep,
                    role,
                    enemies,
                )
            } else if let Some(resource) = find_dropped(&room, 50, Some(ResourceType::Power)).next()
            {
                take_resource(resource.id(), creep, role, enemies)
            } else {
                let _ = creep.suicide();
                TaskResult::Completed
            }
        } else {
            let _ = creep.say("🚬", true);
            TaskResult::StillWorking(Task::PowerbankCarry(pos, id), None)
        }
    } else {
        let goal = Walker::Exploring(true).walk(pos, LONG_RANGE_ACTION, creep, role, enemies);
        TaskResult::StillWorking(Task::PowerbankCarry(pos, id), Some(goal))
    }
}

pub fn find_ruin(room: &Room) -> Option<Ruin> {
    room.find(find::RUINS, None)
        .into_iter()
        .find(|ruin| ruin.store().get_used_capacity(Some(ResourceType::Power)) > 0)
}
