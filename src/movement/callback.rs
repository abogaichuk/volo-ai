use log::*;
use screeps::{
    CostMatrix, LocalRoomTerrain, Position, RoomCoordinate, RoomXY, Terrain,
    constants::StructureType, enums::StructureObject, find, game,
    local::{LocalCostMatrix, RoomName}, pathfinder::{MultiRoomCostResult, SingleRoomCostResult},
    prelude::*
};
use std::collections::{HashMap, HashSet};
use crate::rooms::state::constructions::RoomPart;

pub type SingleRoomCallback = fn(RoomName, CostMatrix) -> SingleRoomCostResult;

#[derive(Debug)]
pub struct PathOptions {
    pub from: Position,
    pub avoid_creeps: bool,
    pub allowed_rooms: HashSet<RoomName>,
    pub danger_zones: Option<(RoomName, Vec<RoomXY>)>
}

pub fn prefer_roads_callback(options: PathOptions)
    -> Box<dyn FnMut(RoomName) -> MultiRoomCostResult>
{
    Box::new(move |room_name: RoomName| -> MultiRoomCostResult {
        //if allowed rooms is not empty -> we have high level route
        if !options.allowed_rooms.is_empty() && !options.allowed_rooms.contains(&room_name) {
            debug!("high level route avoided room: {}", room_name);
            return MultiRoomCostResult::Impassable;
        }
        MultiRoomCostResult::CostMatrix(cost_matrix(room_name, &options, false))
    })
}

pub fn prefer_plain_callback(options: PathOptions)
    -> Box<dyn FnMut(RoomName) -> MultiRoomCostResult>
{
    Box::new(move |room_name: RoomName| -> MultiRoomCostResult {
        //if allowed rooms is not empty -> we have high level route
        if !options.allowed_rooms.is_empty() && !options.allowed_rooms.contains(&room_name) {
            debug!("high level route avoided room: {}", room_name);
            return MultiRoomCostResult::Impassable;
        }
        MultiRoomCostResult::CostMatrix(cost_matrix(room_name, &options, false))
    })
}

pub fn prefer_swamp_callback(options: PathOptions)
    -> Box<dyn FnMut(RoomName) -> MultiRoomCostResult>
{
    Box::new(move |room_name: RoomName| -> MultiRoomCostResult {
        //if allowed rooms is not empty -> we have high level route
        if !options.allowed_rooms.is_empty() && !options.allowed_rooms.contains(&room_name) {
            debug!("high level route avoided room: {}", room_name);
            return MultiRoomCostResult::Impassable;
        }
        MultiRoomCostResult::CostMatrix(cost_matrix(room_name, &options, true))
    })
}

pub fn closest_in_room_range<'a>(grid: &'a HashMap<RoomXY, RoomPart>)
    -> impl FnMut(RoomName, CostMatrix) -> SingleRoomCostResult + use<'a>
{
    |_: RoomName, mut matrix: CostMatrix| -> SingleRoomCostResult {
        for (xy, part) in grid.iter() {
            match part {
                RoomPart::Wall | RoomPart::Exit => { matrix.set_xy(*xy, 0xff) }
                _ => {}
            }
        }
        SingleRoomCostResult::CostMatrix(matrix)
    }
}

pub fn closest_multi_rooms_range() -> impl FnMut(RoomName) -> MultiRoomCostResult
{
    move |_: RoomName| -> MultiRoomCostResult {
        MultiRoomCostResult::CostMatrix(CostMatrix::new())
    }
}

pub fn construction_single_room<'a>(
    unwalkable: HashSet<RoomXY>,
    grid: &'a HashMap<RoomXY, RoomPart>) -> impl FnMut(RoomName) -> MultiRoomCostResult + use<'a>
{
    move |_: RoomName| -> MultiRoomCostResult  {
        let mut matrix = LocalCostMatrix::new();

        for (xy, part) in grid.iter() {
            if unwalkable.contains(xy) { matrix.set_xy(*xy, 0xff) }
            else {
                match part {
                    RoomPart::Wall | RoomPart::Exit => { matrix.set_xy(*xy, 0xff) }
                    _ => {}
                }
            }
        }

        MultiRoomCostResult::CostMatrix(matrix.into())
    }
}

pub fn construction_multi_rooms<'a>(
    planned: &'a HashMap<RoomName, Vec<RoomXY>>,
) -> impl FnMut(RoomName) -> MultiRoomCostResult + use<'a>
{
    move |room_name: RoomName| -> MultiRoomCostResult {
        let mut matrix = LocalCostMatrix::new();
        if let Some(room) = game::rooms().get(room_name) {

            let mut keepers = Vec::new();
            let mut structures = HashSet::new();
            for structure in room.find(find::STRUCTURES, None) {
                if let StructureObject::StructureWall(w) = structure {
                    structures.insert(w.pos().xy());
                } else if let StructureObject::StructureKeeperLair(k) = structure {
                    keepers.push(k.pos().xy());
                }
            }
            if let Some(buildings) = planned.get(&room_name) {
                for xy in buildings.iter() {
                    structures.insert(*xy);
                }
            }

            let terrain = room.get_terrain();
            for y in 0..screeps::ROOM_SIZE {
                for x in 0..screeps::ROOM_SIZE {
                    let xy = unsafe { RoomXY::unchecked_new(x, y) };

                    if structures.contains(&xy) || terrain.get(x, y) == Terrain::Wall {
                        matrix.set(xy, 0xff);
                    } else {
                        let distance = keepers.iter()
                            .map(|keeper| keeper.get_range_to(xy))
                            .min().unwrap_or(u8::MAX);
                        match distance {
                            1 => matrix.set(xy, 0xfa), //250
                            2 => matrix.set(xy, 0xc8), //200
                            3 => matrix.set(xy, 0x05), //5
                            4 => matrix.set(xy, 0x04), //4
                            _ => {}
                        };
                    }
                }
            }
        } else {
            //todo block undesirable rooms in other way
            for y in 0..screeps::ROOM_SIZE {
                for x in 0..screeps::ROOM_SIZE {
                    let xy = unsafe { RoomXY::unchecked_new(x, y) };
                    matrix.set(xy, 0xff);
                }
            }
        }
        MultiRoomCostResult::CostMatrix(matrix.into())
    }
}

fn cost_matrix(room_name: RoomName, options: &PathOptions, prefer_swamp: bool) -> CostMatrix {
    let mut new_matrix = LocalCostMatrix::new();

    if let Some(room) = screeps::game::rooms().get(room_name) {
        for structure in room.find(find::STRUCTURES, None) {
            let pos = structure.pos();
            match structure {
                StructureObject::StructureRoad(_) => {
                    // ignore roads for creeps not needing 'em
                    if new_matrix.get(pos.xy()) == 0 {
                        new_matrix.set(pos.xy(), 0x01);
                    }
                }
                // containers walkable
                StructureObject::StructureContainer(_) => {
                    new_matrix.set(pos.xy(), 0x04); //avoid containers
                }
                StructureObject::StructureWall(_) => {
                    new_matrix.set(pos.xy(), 0xff);
                }
                StructureObject::StructureRampart(rampart) => {
                    // we could check for and path across public ramparts
                    // (and need to do so if we want to enhance this bot to be able
                    // to cross an ally's public ramparts - but for now, simply don't trust 'em
                    if !rampart.my() {
                        new_matrix.set(pos.xy(), 0xff);
                    }
                }
                _ => {
                    // other structures, not walkable
                    new_matrix.set(pos.xy(), 0xff);
                }
            }
        }

        for creep in room.find(find::CREEPS, None) {
            if !creep.my() || (options.avoid_creeps && options.from.get_range_to(creep.pos()) <= 2) {
                new_matrix.set(creep.pos().xy(), 0xff);
            }
        }

        for pc in room.find(find::POWER_CREEPS, None) {
            new_matrix.set(pc.pos().xy(), 0xff);
        }

        for csite in room.find(find::MY_CONSTRUCTION_SITES, None) {
            let pos = csite.pos();
            match csite.structure_type() {
                // walkable structure types
                StructureType::Container | StructureType::Road | StructureType::Rampart => {}
                _ => {
                    // other structures, not walkable
                    new_matrix.set(pos.xy(), 0xff);
                }
            }
        }

        if let Some(danger_zones) = &options.danger_zones && room_name == danger_zones.0 {
                let lrt = LocalRoomTerrain::from(room.get_terrain());
                let penalty = 0xa;

                //trying to avoid attackable cells
                for xy in danger_zones.1.iter() {
                    match new_matrix.get(*xy) {
                        0xff => {} //cell is unwalkable -> do nothing
                        1 => {
                            //road here
                            if !prefer_swamp {
                                new_matrix.set(*xy, penalty / 2) //reduced penalty set on cell with road
                            }
                        }
                        cost => { //no road here
                            match lrt.get_xy(*xy) {
                                Terrain::Wall => new_matrix.set(*xy, 0xff),
                                Terrain::Swamp => {
                                    if prefer_swamp {
                                        new_matrix.set(*xy, cost + penalty) //just penalty
                                    } else {
                                        new_matrix.set(*xy, cost + 0xa + penalty) //swamp cost + penalty
                                    }
                                },
                                Terrain::Plain => new_matrix.set(*xy, cost + 0x02 + penalty) //plain cost + penalty
                            }
                        }
                    }
                }
            }
    }

    new_matrix.into()
}