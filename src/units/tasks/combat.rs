use log::*;
use screeps::{
    find, game, Attackable, Creep, HasPosition, ObjectId, Part, Position,
    Room, RoomCoordinate, RoomName, SharedCreepProperties, StructureInvaderCore,
    StructureKeeperLair, StructureObject, StructureRampart, StructureTower,
    SOURCE_KEEPER_USERNAME, INVADER_USERNAME, SYSTEM_USERNAME
};
use std::cmp::Ordering;
use crate::{
    movement::{walker::Walker, MovementGoal},
    units::{Task, TaskResult, roles::Role, has_part, with_parts},
    utils::{
        commons::*,
        constants::{CLOSE_RANGE_ACTION, LONG_RANGE_ACTION}
    }
};

//todo chase into another room task !
pub fn defend(room_name: RoomName, room_requested: bool, creep: &Creep, role: &Role, hostiles: Vec<Creep>) -> TaskResult {
    if creep.pos().room_name() != room_name {
        TaskResult::RunAnother(Task::MoveMe(room_name, Walker::Aggressive))
    } else if let Some(attacker) = closest_attacker(creep, hostiles.iter()) {
        debug!("{} hostiles is not empty move to room_name: {}", creep.name(), room_name);
        let goal = combat(creep, role, attacker, &hostiles);
        TaskResult::StillWorking(Task::Defend(room_name, room_requested), goal)
    } else if room_requested {
        TaskResult::ResolveRequest(Task::Defend(room_name, room_requested), false)
    } else if let Some(any_not_ally) = hostiles.first() {
        let goal = combat(creep, role, any_not_ally, &hostiles);
        TaskResult::StillWorking(Task::Defend(room_name, room_requested), goal)
    } else if let Some(injured) = find_closest_injured_my_creeps(creep) {
        let goal = Walker::Therapeutic.walk(injured.pos(), 0, creep, role, hostiles);
        TaskResult::StillWorking(Task::Defend(room_name, room_requested), Some(goal))
    } else {
        let _ = creep.say("🚬", false);
        let _ = creep.heal(creep);
        TaskResult::Completed
    }
}

pub fn oversee(room_name: RoomName, target: Option<(Position, u32)>, creep: &Creep, role: &Role, hostiles: Vec<Creep>) -> TaskResult {
    let room = creep.room().expect("expect creep is in a room!");

    if creep.pos().room_name() != room_name {
        //if creep is not in target room -> go to target or to the closest exit to this room
        if let Some(target) = target {
            let goal = Walker::Aggressive.walk(target.0, target.1, creep, role, hostiles);
            TaskResult::StillWorking(Task::Oversee(room_name, Some(target)), Some(goal))
        } else {
            TaskResult::RunAnother(Task::MoveMe(room_name, Walker::Aggressive))
        }
    } else if let Some(closest_sk) = find_closest_source_keeper_guard(creep.pos(), &hostiles) {
        if creep.pos().is_near_to(closest_sk.pos()) {
            let goal = Walker::Aggressive.walk(closest_sk.pos(), 0, creep, role, hostiles.clone());
            TaskResult::StillWorking(Task::Oversee(room_name, Some((closest_sk.pos(), 0))), Some(goal))
        } else {
            let goal = Walker::Aggressive.walk(closest_sk.pos(), 1, creep, role, hostiles.clone());
            TaskResult::StillWorking(Task::Oversee(room_name, Some((closest_sk.pos(), 1))), Some(goal))
        }
    } else if let Some(keeper_lair) = find_fastest_keeper_lair_spawn(&room) {
        //if no source keeper guards -> go to a keeper lair
        let goal = Walker::Aggressive.walk(keeper_lair.pos(), 1, creep, role, hostiles);
        TaskResult::StillWorking(Task::Oversee(room_name, Some((keeper_lair.pos(), 1))), Some(goal))
    } else {
        warn!("{} there is no keeper_lairs in {}", creep.name(), room_name);
        let _ = creep.say("🚬", false);
        TaskResult::Completed
    }
}

//todo consume many cpu > 450 cpu, overseer
pub fn find_heal(room_name: RoomName, creep: &Creep, role: &Role, hostiles: Vec<Creep>) -> TaskResult {
    if creep.pos().room_name() != room_name {
        TaskResult::RunAnother(Task::MoveMe(room_name, Walker::Aggressive))
    } else if let Some(injured) = find_closest_injured_my_creeps(creep) {
        let goal = Walker::Aggressive.walk(injured.pos(), 0, creep, role, hostiles);
        TaskResult::StillWorking(Task::FindHeal(room_name), Some(goal))
    } else {
        let _ = creep.say("🚬", false);
        TaskResult::Completed
    }
}

pub fn crash(id: ObjectId<StructureInvaderCore>, pos: Position, creep: &Creep, role: &Role, hostiles: Vec<Creep>) -> TaskResult {
    let attackers = with_parts(hostiles, vec![Part::RangedAttack]);

    if pos.room_name() != creep.pos().room_name() {
        TaskResult::RunAnother(Task::MoveMe(pos.room_name(), Walker::Aggressive))
    }
    //todo ignore enemies???
    // else if !attackers.is_empty() {
    //     //if in a target room and enemies exist -> escape
    //     if let Some(closest_exit) = find_closest_exit(creep, None) {
    //         let goal = Walker::Aggressive.walk(closest_exit, 0, creep, role, attackers);
    //         TaskResult::StillWorking(Task::Escape(closest_exit), Some(goal))
    //     } else {
    //         warn!("{} no exit found in room {}", creep.name(), creep.pos().room_name());
    //         TaskResult::Abort
    //     }
    // }
    else if !creep.pos().is_near_to(pos) {
        //if in a target room and there is no enemies, but far away from invander core -> move to 
        let goal = Walker::Aggressive.walk(pos, 1, creep, role, attackers);
        TaskResult::StillWorking(Task::Crash(id, pos), Some(goal))
    } else if let Some(ic) = id.resolve() {
        //if near target pos and ic exist -> attack
        let _ = creep.say("🥊", true);
        let _ = creep.attack(&ic);
        TaskResult::StillWorking(Task::Crash(id, pos), None)
    } else {
        TaskResult::ResolveRequest(Task::Crash(id, pos), true)
    }
}

pub fn defend_home(creep: &Creep, role: &Role, hostiles: Vec<Creep>) -> TaskResult {
    if let Some(in_range) = hostiles.iter()
        .find(|hostile| hostile.pos().is_near_to(creep.pos()))
    {
        let _ = creep.attack(in_range);
    }
    let home_room = role.get_home()
        .and_then(|home| game::rooms().get(home))
        .expect("expect role has a home!");

    let goal = get_closest_walkable_rampart(creep.pos(), &home_room, &hostiles)
        .map(|rampart| Walker::Aggressive.walk(rampart.pos(), 0, creep, role, hostiles));
    TaskResult::StillWorking(Task::DefendHome, goal)
}

fn any_caravan_cargo(hostiles: &[Creep]) -> Option<Position> {
    hostiles.iter()
        .find(|hostile| hostile.owner().username() == SYSTEM_USERNAME && hostile.store().get_used_capacity(None) > 0)
        .map(|cargo| cargo.pos())
}

pub fn protect(room_name: RoomName, target_pos: Option<Position>, creep: &Creep, role: &Role, hostiles: Vec<Creep>, structures: Vec<StructureObject>) -> TaskResult {
    // let in_range: Vec<&Creep> = hostiles.iter().filter(|hostile| creep.pos().get_range_to(hostile.pos()) <= 4).collect();
    if creep.hits() == creep.hits_max() {
        if creep.pos().room_name() != room_name {
            //in any room check for caravan existance
            if let Some(cargo_pos) = any_caravan_cargo(&hostiles) {
                let goal = caravan_combat(creep, role, hostiles, cargo_pos);
                TaskResult::StillWorking(Task::Protect(room_name, goal.as_ref().map(|goal| goal.pos)), goal)
            } else if let Some(target) = target_pos && !creep.pos().is_near_to(target) {
                //with full hp in a different room -> go to target room
                let goal = Walker::Aggressive.walk(target, 1, creep, role, hostiles);
                TaskResult::StillWorking(Task::Protect(room_name, Some(target)), Some(goal))
            } else {
                TaskResult::RunAnother(Task::MoveMe(room_name, Walker::Aggressive))
            }
        }
        //with full hp in a target room
        else if let Some(tower) = closest_tower(creep, &structures) {
            if creep.pos().get_range_to(tower.pos()) <= 3 {
                let _ = creep.ranged_attack(tower);
            } else if let Some(any) = any_in_range_structure(creep, &structures) {
                let _ = creep.ranged_attack(any);
            }
            let goal = Walker::Berserk.walk(tower.pos(), CLOSE_RANGE_ACTION, creep, role, hostiles);
            TaskResult::StillWorking(Task::Protect(room_name, Some(tower.pos())), Some(goal))
        } else if let Some(spawn) = closest_spawn_or_ext(creep, &structures) {
            let goal = Walker::Berserk.walk(spawn.pos(), CLOSE_RANGE_ACTION, creep, role, hostiles);
            TaskResult::StillWorking(Task::Protect(room_name, Some(spawn.pos())), Some(goal))
        } else if !hostiles.is_empty() {
            if let Some(cargo_pos) = any_caravan_cargo(&hostiles) {
                let goal = caravan_combat(creep, role, hostiles, cargo_pos);
                TaskResult::StillWorking(Task::Protect(room_name, goal.as_ref().map(|goal| goal.pos)), goal)
            } else if let Some(closest) = closest_creep(creep, hostiles.iter()
                .filter(|hostile| hostile.body().len() > 2))
                // .filter(|hostile| BLACK_LIST.contains(&hostile.owner().username().as_str())))
            {
                let goal = combat(creep, role, closest, &hostiles);
                TaskResult::StillWorking(Task::Protect(room_name, goal.as_ref().map(|goal| goal.pos)), goal)
            } else {
                let _ = creep.say("🚬", true);
                TaskResult::Completed
            }
        } else if let Some(target) = target_pos && !creep.pos().is_equal_to(target) {
            //if caravan moved to another room got to saved target pos
            let goal = Walker::Aggressive.walk(target, 0, creep, role, hostiles);
            TaskResult::StillWorking(Task::Protect(room_name, Some(target)), Some(goal))
        } else if let Some(injured) = find_closest_injured_my_creeps(creep) {
            let goal = Walker::Therapeutic.walk(injured.pos(), 0, creep, role, hostiles);
            TaskResult::StillWorking(Task::Protect(room_name, None), Some(goal))
        } else {
            let _ = creep.say("🚬", false);
            TaskResult::Completed
        }
    } else {
        //injured
        if creep.pos().room_name() != room_name {
            //injured outside the target room, wait for heal
            if creep.pos().is_room_edge() && let Some(pos) = find_walkable_positions_near_by(creep.pos(), true).first() {
                let goal = Walker::Aggressive.walk(*pos, 0, creep, role, hostiles);
                TaskResult::StillWorking(Task::Protect(room_name, None), Some(goal))
            } else {
                let _ = creep.heal(creep);
                let _ = creep.ranged_mass_attack();

                TaskResult::StillWorking(Task::Protect(room_name, None), None)
                //just wait
            }
        } else {
            //injured in a target room -> run away
            let closest_exit = creep.pos().find_closest_by_path(find::EXIT, None)
                .map(|rp| rp.into())
                .unwrap_or(Position::new(
                    unsafe { RoomCoordinate::unchecked_new(25) },
                    unsafe { RoomCoordinate::unchecked_new(25) },
                    role.get_home().expect("expect home room")));
            
            if let Some(any) = any_in_range_structure(creep, &structures) {
                let _ = creep.ranged_attack(any);
            }

            let goal = Walker::Berserk.walk(closest_exit, 0, creep, role, hostiles);
            TaskResult::StillWorking(Task::Protect(room_name, None), Some(goal))
        }
    }
}

fn caravan_combat(creep: &Creep, role: &Role, hostiles: Vec<Creep>, cargo_pos: Position) -> Option<MovementGoal> {
    if let Some(closest_healer) = closest_caravan_healer(cargo_pos, hostiles.iter()) {
        info!("{} move to closest_healer: {}", creep.name(), closest_healer.pos());
        Some(Walker::Berserk.walk(closest_healer.pos(), 0, creep, role, hostiles))
    } else if let Some(closest_cargo) = closest_caravan_cargo(creep.pos(), hostiles.iter()) {
        info!("{} move to closest closest_cargo: {}", creep.name(), closest_cargo.pos());
        Some(Walker::Berserk.walk(closest_cargo.pos(), 0, creep, role, hostiles))
    } else if let Some(any_caravan) = closest_creep(creep, hostiles.iter()) {
        info!("{} move to closest closest_creep: {}", creep.name(), any_caravan.pos());
        combat(creep, role, any_caravan, &hostiles)
    } else {
        None
    }
}

//todo a bug, sometimes heal injured instead of attack enemy
fn combat(creep: &Creep, role: &Role, closest: &Creep, hostiles: &[Creep]) -> Option<MovementGoal> {
    let in_range_list: Vec<&Creep> = hostiles.iter()
        .filter(|hostile| creep.pos().in_range_to(hostile.pos(), LONG_RANGE_ACTION))
        .collect();

    if in_range_list.len() > 2 {
        let _ = creep.ranged_mass_attack();
        let _ = creep.heal(creep);
    } else if creep.pos().get_range_to(closest.pos()) <= 3 {
        let _ = creep.ranged_attack(closest);
        let _ = creep.heal(creep);
    } else if let Some(enemy) = in_range_list.first() {
        let _ = creep.ranged_attack(*enemy);
        let _ = creep.heal(creep);
    } else {
        try_heal(creep);
    }

    let with_attack_part = has_part(&[Part::Attack], closest, true);
    match creep.pos().get_range_to(closest.pos()) {
        1 if closest.owner().username() != INVADER_USERNAME && with_attack_part => {
            let goal = Walker::Flee.walk(closest.pos(), LONG_RANGE_ACTION, creep, role, Vec::new());
            Some(goal)
        }
        1 => Some(Walker::Exploring(false).walk(closest.pos(), 0, creep, role, Vec::new())),
        2 | 3 if closest.owner().username() != INVADER_USERNAME && with_attack_part => None, //just stay
        _ => Some(Walker::Exploring(false).walk(closest.pos(), 1, creep, role, Vec::new()))
    }
}

fn get_closest_walkable_rampart(to: Position, home_room: &Room, boosted_enemies: &[Creep]) -> Option<StructureRampart> {
    find_ramparts(home_room)
        .filter(|rampart| rampart.pos().is_equal_to(to) || is_walkable(&rampart.pos()))
        .filter_map(|rampart| min_distance(rampart, boosted_enemies))
        .reduce(|acc, item| {
            if item.1 < acc.1 {
                item
            } else {
                acc
            }
        })
        .map(|item| item.0)
}

fn min_distance(rampart: StructureRampart, boosted_enemies: &[Creep]) -> Option<(StructureRampart, u32)> {
    boosted_enemies.iter()
        .map(|enemy| rampart.pos().get_range_to(enemy.pos())).min()
        .map(|distance| (rampart, distance))
}

fn find_closest_source_keeper_guard(to: Position, hostiles: &[Creep]) -> Option<&Creep> {
    hostiles.iter()
        .filter(|hostile| hostile.owner().username() == SOURCE_KEEPER_USERNAME)
        .reduce(|acc, item| {
            if to.get_range_to(item.pos()) < to.get_range_to(acc.pos()) {
                item
            } else {
                acc
            }
        })
}

fn find_fastest_keeper_lair_spawn(room: &Room) -> Option<StructureKeeperLair> {
    find_keeper_lairs(room)
        .reduce(|acc, item| {
            if item.ticks_to_spawn() < acc.ticks_to_spawn() {
                item
            } else {
                acc
            }
        })
}

fn closest_tower<'a>(to: &'a dyn HasPosition, structures: &'a [StructureObject]) -> Option<&'a StructureTower> {
    structures
        .iter()
        .filter_map(|structure| {
            match structure {
                StructureObject::StructureTower(t) => Some(t),
                _ => None
            }
        })
        .reduce(|acc, t: &StructureTower| {
            if to.pos().get_range_to(t.pos()) < to.pos().get_range_to(acc.pos()) {
                t
            } else {
                acc
            }
        })
}

fn closest_spawn_or_ext<'a>(to: &'a dyn HasPosition, structures: &'a [StructureObject]) -> Option<&'a StructureObject> {
    structures.iter()
        .filter(|structure| {
            matches!(structure, StructureObject::StructureSpawn(_) | StructureObject::StructureExtension(_))
        })
        .reduce(|acc, t: &StructureObject| {
            if to.pos().get_range_to(t.pos()) < to.pos().get_range_to(acc.pos()) {
                t
            } else {
                acc
            }
        })
}

fn any_in_range_structure<'a>(to: &'a dyn HasPosition, structures: &'a [StructureObject]) -> Option<&'a dyn Attackable> {
    structures.iter()
        .filter(|structure| structure.pos().get_range_to(to.pos()) <= 3)
        .find_map(|structure| structure.as_attackable())
}

fn closest_caravan_cargo<'a>(
    to: Position,
    iterator: impl Iterator<Item = &'a Creep>) -> Option<&'a Creep>
{
    iterator
        .filter(|enemy| enemy.owner().username() == SYSTEM_USERNAME
            && enemy.store().get_used_capacity(None) > 0)
        .fold(None, |acc, another| {
            if let Some(creep) = acc {
                match another.pos().get_range_to(to).cmp(&creep.pos().get_range_to(to)) {
                    Ordering::Less => Some(another),
                    _ => Some(creep),
                }
            } else {
                Some(another)
            }
        })
}

fn closest_caravan_healer<'a>(
    to: Position,
    iterator: impl Iterator<Item = &'a Creep>) -> Option<&'a Creep>
{
    iterator
        .filter(|enemy| enemy.owner().username() == SYSTEM_USERNAME &&
            has_part(&[Part::Heal], enemy, false) &&
            !has_part(&[Part::RangedAttack, Part::Attack, Part::Carry], enemy, false))
        .fold(None, |acc, another| {
            if let Some(creep) = acc {
                match another.pos().get_range_to(to).cmp(&creep.pos().get_range_to(to)) {
                    Ordering::Less => Some(another),
                    _ => Some(creep),
                }
            } else {
                Some(another)
            }
        })
}