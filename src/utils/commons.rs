use log::*;
use screeps::{
    ConstructionSite, Creep, Flag, HasId, HasPosition, HasStore,
    OwnedStructureProperties, Part, Position, ROOM_SIZE, RawObjectId,
    Resource, ResourceType, Room, RoomCoordinate, RoomName, RoomPosition,
    RoomXY, SharedCreepProperties, Source, StructureFactory, StructureKeeperLair,
    StructureObject, StructureRampart, StructureStorage, StructureTerminal,
    StructureType, Terrain, find, game, look::{LookResult, STRUCTURES}
};
use crate::{
    units::{Memory, roles::Role},
    utils::constants::{ROOM_NUMBER_RE, HIGH_CPU_THRESHOLD, LOW_BUCKET_THRESHOLD}
};
use std::{cmp::Ordering, str::FromStr, iter::{Iterator, Empty, Once}};
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;

pub enum Either<A, B, C> {
    A(A),
    B(B),
    C(C)
}

impl<T, A, B, C> Iterator for Either<A, B, C>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
{
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Either::A(a) => a.next(),
            Either::B(b) => b.next(),
            Either::C(c) => c.next(),
        }
    }
}

impl<T> From<Option<T>>
    for Either<std::option::IntoIter<T>, Empty<T>, Once<T>>
{
    fn from(opt: Option<T>) -> Self {
        // Single concrete type whether Some or None
        Either::A(opt.into_iter())
    }
}

pub fn find_ramparts(room: &Room) -> impl Iterator<Item = StructureRampart> {
    room.find(find::MY_STRUCTURES, None)
        .into_iter()
        .filter_map(|structure| {
            match structure {
                StructureObject::StructureRampart(r) => Some(r),
                _ => None
            }
        })
}

pub fn is_cpu_on_low() -> bool {
    let used_cpu = game::cpu::get_used();
    let bucket_cpu = game::cpu::bucket();

    if used_cpu > HIGH_CPU_THRESHOLD {
        debug!(
            "CPU usage high, will skip finding fresh paths: {}",
            used_cpu
        );
        true
    } else if bucket_cpu < LOW_BUCKET_THRESHOLD {
        debug!(
            "CPU bucket low, will skip finding fresh paths: {}",
            bucket_cpu
        );
        true
    } else {
        false
    }
}

pub fn get_random(from: usize, to: usize) -> usize {
    rand::thread_rng().gen_range(from..to)
}

pub fn find_keeper_lairs(room: &Room) -> impl Iterator<Item = StructureKeeperLair> {
    room.find(find::HOSTILE_STRUCTURES, None)
        .into_iter()
        .filter_map(|structure| match structure {
            StructureObject::StructureKeeperLair(kl) => Some(kl),
            _ => None
        })
}

pub fn find_closest_injured_my_creeps(creep: &Creep) -> Option<Creep> {
    let room = creep.room().expect("couldn't resolve a room");

    let mut injured:Vec<Creep> = find_injured_my_creeps(&room, Some(creep.name())).collect();

    injured.sort_by_key(|friend| friend.pos().get_range_to(creep.pos()));
    injured.reverse();

    injured.pop()
}

pub fn find_injured_my_creeps(room: &Room, ignore: Option<String>) -> impl Iterator<Item = Creep> {
    room.find(find::MY_CREEPS, None)
        .into_iter()
        .filter(move |creep| ignore.as_ref()
            .is_some_and(|name| *name != creep.name()) && creep.hits() < creep.hits_max())
}

pub fn find_exit(to_room: RoomName, creep: &Creep, room: &Room) -> Option<Position> {
    if room.name() == to_room {
        warn!("fix me: {}, roomname: {}, to_room: {}", creep.name(), room.name(), to_room);
        None
    } else {
        room.find_exit_to(to_room).ok()
            .and_then(|exit_direction| creep.pos()
                .find_closest_by_path(find::Exit::from(exit_direction), None)
                .map(|rp| rp.into()))
    }
}

pub fn capture_room_numbers(re: &Regex, room_name: RoomName) -> Option<(u32, u32)> {
    let room_str = room_name.to_string();
    re.captures(&room_str)
        .and_then(|caps| {
            let first = <u32 as FromStr>::from_str(&caps[1]).ok()?;
            let second = <u32 as FromStr>::from_str(&caps[2]).ok()?;
            Some((first, second))
        })
}

pub fn say_message(creep: &Creep) {
    let word = game::time() % 10;
    let _ = match word {
        1 => creep.say("dnt do it!", true),
        2 => creep.say("b careful!", true),
        3 => creep.say("u can hurt", true),
        4 => creep.say("yourself!", true),
        5 => creep.say("no reasons", true),
        6 => creep.say("for suicide!", true),
        7 => creep.say("man!", true),
        8 => creep.say("let's build", true),
        9 => creep.say("safe world", true),
        0 => creep.say("togather!", true),
        _ => Ok(())
    };
}

pub fn closest_attacker<'a>(
    to: &dyn HasPosition,
    iterator: impl Iterator<Item = &'a Creep>) -> Option<&'a Creep>
{
    iterator
        // .filter(|enemy| has_part(enemy, Part::Attack) || has_part(enemy, Part::RangedAttack))
        .filter(|hostile| has_part(&[Part::Attack, Part::RangedAttack], hostile, false))
        .fold(None, |acc, another| {
            if let Some(hostile) = acc {
                match another.pos().get_range_to(to.pos()).cmp(&hostile.pos().get_range_to(to.pos())) {
                    Ordering::Less => Some(another),
                    Ordering::Greater => Some(hostile),
                    Ordering::Equal => {
                        if has_part(&[Part::Attack], hostile, true) {
                            Some(hostile)
                        } else {
                            Some(another)
                        }
                    }
                }
            } else {
                Some(another)
            }
        })
}

pub fn closest_creep<'a>(
    to: &dyn HasPosition,
    iterator: impl Iterator<Item = &'a Creep>) -> Option<&'a Creep>
{
    iterator.fold(None, |acc, another| {
        if let Some(creep) = acc {
            match another.pos().get_range_to(to.pos()).cmp(&creep.pos().get_range_to(to.pos())) {
                Ordering::Less => Some(another),
                Ordering::Greater => Some(creep),
                Ordering::Equal => {
                    if has_part(&[Part::Attack], creep, true) {
                        Some(creep)
                    } else {
                        Some(another)
                    }
                }
            }
        } else {
            Some(another)
        }
    })
}

pub fn find_container_near_by(to: &dyn HasPosition, range: u8, str_types: &[StructureType]) -> Option<StructureObject> {
    to.pos().find_in_range(find::STRUCTURES, range)
        .into_iter()
        .find(|str| str_types.contains(&str.structure_type()))
        // .find(|str| str.structure_type() == StructureType::Link || str.structure_type() == StructureType::Container)
}

pub fn find_cs_near_by(to: &dyn HasPosition, range: u8) -> Option<ConstructionSite> {
    to.pos().find_in_range(find::CONSTRUCTION_SITES, range).into_iter()
        .filter(|cs| cs.my() && (cs.structure_type() == StructureType::Container || cs.structure_type() == StructureType::Road))
        .reduce(|acc, elem| {
            if acc.pos().get_range_to(to.pos()) > elem.pos().get_range_to(to.pos()) {
                acc
            } else {
                elem
            }
        })
}

pub fn in_range_to<'a>(to: &dyn HasPosition, hostiles: impl Iterator<Item = &'a Creep>, range: u32) -> usize {
    hostiles
        .filter(|hostile| to.pos().in_range_to(hostile.pos(), range))
        .count()
}

pub fn has_part(parts: &[Part], creep: &Creep, is_active: bool) -> bool {
    creep.body().iter()
        .filter(|bodypart| !is_active || bodypart.hits() > 0) //if is_active filter only bodyparts with hits
        .any(|bodypart| parts.contains(&bodypart.part()))
}

pub fn get_compressed_resource(resource: ResourceType) -> Option<ResourceType> {
    resource
        .commodity_recipe()
        .and_then(|recipe| recipe.components.iter()
            .find_map(|(component, _)| {
                if *component != ResourceType::Energy {
                    Some(*component)
                } else {
                    None
                }
            }))
}

pub fn find_hostiles(room: &Room, parts: Vec<Part>) -> impl Iterator<Item = Creep> {
    room.find(find::HOSTILE_CREEPS, None)
        .into_iter()
        .filter(move |creep| {
            creep.body().iter()
                .map(|bodypart| bodypart.part())
                .any(|part| parts.is_empty() || parts.contains(&part))
        })
}

pub fn find_hostiles_nearby<'a>(room: &Room, parts: Vec<Part>, to: &'a dyn HasPosition) -> impl Iterator<Item = Creep> + use<'a> {
    room.find(find::HOSTILE_CREEPS, None)
        .into_iter()
        .filter(move |creep| {
            creep.pos().is_near_to(to.pos()) && creep.body().iter()
                .any(|bodypart| bodypart.hits() > 0 && (parts.is_empty() || parts.contains(&bodypart.part())))
        })
}

pub fn full_boosted(creep: &Creep) -> bool {
    creep.body().iter().all(|bodypart| bodypart.boost().is_some())
}

pub fn is_boosted(creep: &Creep) -> bool {
    creep.body().iter().any(|bodypart| bodypart.boost().is_some())
}

pub fn is_under_rampart(position: RoomPosition) -> bool {
    position.look_for(STRUCTURES).ok()
        .is_some_and(|structure| structure.iter()
            .any(|structure| structure.as_structure().structure_type() == StructureType::Rampart))
}

pub fn look_for(position: &RoomPosition, structure_type: StructureType) -> bool {
    position.look_for(STRUCTURES).ok()
        .is_some_and(|structure| structure.iter()
            .any(|structure| structure.as_structure().structure_type() == structure_type))
}

pub fn find_container_with(
        resource: ResourceType,
        amount: Option<u32>,
        storage: Option<&StructureStorage>,
        terminal: Option<&StructureTerminal>,
        factory: Option<&StructureFactory>)
    -> Option<(RawObjectId, u32)>
{
    let amount = amount.unwrap_or_default();
    storage
        .and_then(|storage| {
            let used_amount = storage.store().get_used_capacity(Some(resource));
            if used_amount >= amount && used_amount > 0 { Some((storage.raw_id(), used_amount)) } else { None }
        })
        .or_else(|| terminal
            .and_then(|terminal| {
                let used_amount = terminal.store().get_used_capacity(Some(resource));
                if used_amount >= amount && used_amount > 0 { Some((terminal.raw_id(), used_amount)) } else { None }
            })
                .or_else(|| factory
                    .and_then(|factory| {
                        let used_amount = factory.store().get_used_capacity(Some(resource));
                        if used_amount >= amount && used_amount > 0 { Some((factory.raw_id(), used_amount)) } else { None }
                    })))
}

pub fn get_positions_near_by(position: Position, range: u8, exclude_current: bool, exclude_edge: bool) -> Vec<(u8, u8)> {
    let mut result: Vec<(u8, u8)> = Vec::new();

    let start_x = if position.x().u8() <= range { 0 } else { position.x().u8() - range };
    let start_y = if position.y().u8() <= range { 0 } else { position.y().u8() - range };

    debug!("start_x: {}, start_y: {}", start_x, start_y);

    for x in start_x..=position.x().u8() + range {
        for y in start_y..=position.y().u8() + range {
            if x > 49 || y > 49
                || (exclude_current && x == position.x().u8() && y == position.y().u8())
                || (exclude_edge && (x == 0 || x == 49 || y == 0 || y == 49)) {
                    continue;
            }
            result.extend_one((x, y));
        }
    }
    result
}

pub fn is_near_edge(position: Position) -> bool {
    position.x().u8() == 1 || position.x().u8() == ROOM_SIZE -2 || position.y().u8() == 1 || position.y().u8() == ROOM_SIZE -2
}

pub fn remoted_from_edge(position: Position, range: u8) -> bool {
    if range > ROOM_SIZE {
        false
    } else {
        !(position.x().u8() < range || position.x().u8() > ROOM_SIZE - range || position.y().u8() < range || position.y().u8() > ROOM_SIZE - range)
    }
}

pub fn find_walkable_positions_near_by(position: Position, exclude_edge: bool) -> Vec<Position> {
    get_positions_near_by(position, 1, true, exclude_edge)
        .into_iter()
        .map(|elem| RoomPosition::new(elem.0, elem.1, position.room_name()).into())
        .filter(is_walkable)
        .collect::<Vec<Position>>()
}

pub fn get_in_room_bank(room: &Room) -> Option<StructureObject> {
    if let Some(storage) = room.storage() {
        if storage.store().get_free_capacity(None) >= 5000 {
            return Some(storage.into())
        }
    }

    if let Some(terminal) = room.terminal() {
        if terminal.store().get_free_capacity(None) >= 5000 {
            return Some(terminal.into())
        }
    }

    room.find(find::STRUCTURES, None)
        .into_iter()
        .find(|s| {
            let s_type = s.as_structure().structure_type();
            s_type == StructureType::Container
        })
}

pub fn is_walkable(position: &Position) -> bool {
    match position.look() {
        Ok(results) => results.iter()
            .all(|look_result| {
                match look_result {
                    LookResult::Creep(_) => false,
                    LookResult::PowerCreep(_) => false,
                    LookResult::Deposit(_) => false,
                    LookResult::Mineral(_) => false,
                    // LookResult::ScoreCollector(_) => false,
                    // LookResult::ScoreContainer(_) => false,
                    LookResult::Structure(s) => {
                        match StructureObject::from(s.to_owned()) {
                            StructureObject::StructureRampart(rampart) => rampart.my(),
                            StructureObject::StructureRoad(_) => true,
                            _ => false
            
                        }
                    },
                    LookResult::Terrain(terrain) => !matches!(terrain, Terrain::Wall),
                    _ => true
                }
            }), 
        Err(_) => {
            // error!("look result error: {:?}", err);
            false
        }
    }
}

pub fn find_closest_exit(creep: &Creep, to: Option<RoomName>) -> Option<Position> {
    let room = creep.room().expect("expect creep is in a room!");
    let exit = to
        .and_then(|to_room| room.find_exit_to(to_room).ok())
        .map(find::Exit::from)
        .unwrap_or(find::Exit::All);

    creep.pos()
        .find_closest_by_path(exit, None)
        .map(|p| p.into())
}

pub fn find_source_near(pos: Position, room: &Room) -> Option<Source> {
    room.find(find::SOURCES, None).into_iter()
        .find(|source| pos.is_near_to(source.pos()))
}

pub fn has_enough_space(container: &dyn HasStore, amount: u32) -> bool {
    container.store().get_free_capacity(None) >= amount as i32
}

pub fn get_place_to_store(room: &Room) -> Option<StructureObject> {
    room.storage()
        .filter(|storage| storage.store().get_free_capacity(None) > 10000)
        .map(StructureObject::StructureStorage)
        .or_else(|| room.terminal().map(StructureObject::StructureTerminal))
}

pub fn find_dropped(room: &Room, resource_threshold: u32, resource_type: Option<ResourceType>) -> impl Iterator<Item = Resource> {
    room.find(find::DROPPED_RESOURCES, None)
        .into_iter()
        .filter(move |resource| resource.amount() > resource_threshold)
        .filter(move |resource| resource_type.is_none_or(|searching_type| searching_type == resource.resource_type()))
}

pub fn find_flags(room: &Room) -> Vec<Flag> {
    room.find(find::FLAGS, None)
}

pub fn try_heal(creep: &Creep) {
    match find_closest_injured(creep) {
        Some(injured) => {
            match creep.pos().get_range_to(injured.pos()) {
                0 => { let _ = creep.heal(creep); }
                1 => { let _ = creep.heal(&injured); }
                2 | 3 => { let _ = creep.ranged_heal(&injured); }
                _ => { let _ = creep.heal(creep); }
            };
        }
        _ => { let _ = creep.heal(creep); }
    };
}

pub fn find_closest_injured(to: &Creep) -> Option<Creep> {
    let room = to.room().expect("expect creep is in a room!");
    room.find(find::MY_CREEPS, None)
        .into_iter()
        .filter(|creep| creep.hits() < creep.hits_max())
        .reduce(|acc, item| {
            if to.pos().get_range_to(item.pos()) < to.pos().get_range_to(acc.pos()) {
                item
            } else {
                acc
            }
        })
}

pub fn find_roles(role: &Role, in_spawn: &[Role], alive: &HashMap<String, Memory>) -> usize {
    in_spawn
        .iter()
        .filter(|future_creep| *future_creep == role)
        .chain(alive.values()
            .map(|mem| &mem.role)
            .filter(|r: &&Role| *r == role))
        .count()
}

pub fn get_room_regex() -> Regex {
    Regex::new(ROOM_NUMBER_RE).expect("expect regex is valid")
}

pub fn capture_room_parts(re: &Regex, room_name: RoomName) -> Option<(u32, u32)> {
    let room_str = room_name.to_string();

    re.captures(&room_str)
        .and_then(|caps| {
            let first = <u32 as FromStr>::from_str(&caps[1]).ok()?;
            let second = <u32 as FromStr>::from_str(&caps[2]).ok()?;
            Some((first % 10, second % 10))
        })
}

pub fn is_highway(f_mod: u32, s_mod: u32) -> bool {
    f_mod == 0 || s_mod == 0
}

pub fn is_cross_road(f_mod: u32, s_mod: u32) -> bool {
    f_mod == 0 && s_mod == 0
}

pub fn is_central(f_mod: u32, s_mod: u32) -> bool {
    f_mod == 5 && s_mod == 5
}

pub fn is_skr_walkway(f_rem: u32, s_rem: u32) -> bool {
    (f_rem == 5 && (s_rem == 4 || s_rem == 6)) || (s_rem == 5 && (f_rem == 4 || f_rem == 6))
}

pub fn is_skr(f_mod: u32, s_mod: u32) -> bool {
    if is_central(f_mod, s_mod) {
        false
    } else {
        (4..=6).contains(&f_mod) && (4..=6).contains(&s_mod)
    }
}

pub fn room_xy(x: u8, y: u8) -> RoomXY {
    unsafe { RoomXY::new(RoomCoordinate::unchecked_new(x), RoomCoordinate::unchecked_new(y)) }
}