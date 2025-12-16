use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::mem;

use itertools::{Either, Itertools};
use screeps::ROOM_SIZE;
use screeps::constants::Terrain;
use screeps::local::RoomXY;
use screeps::{Position, ResourceType, RoomName, StructureType};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod farm;
mod owned;
mod xy_util;

type OuterRectangle = (u8, u8, u8, u8);
type Walls = [[bool; ROOM_SIZE as usize]; ROOM_SIZE as usize];
type Sat = [[u16; ROOM_SIZE as usize]; ROOM_SIZE as usize];

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RoomPlannerError {
    #[error("low cpu")]
    LowCPU,
    #[error("controller not found")]
    ControllerNotFound,
    #[error("storage not found")]
    StorageNotFound,
    #[error("mineral not found")]
    MineralNotFound,
    #[error("could not place structures near controller")]
    ControllerPlacementFailure,
    #[error("could not place structures near source")]
    SourcePlacementFailure,
    #[error("could not place container near mineral")]
    MineralPlacementFailure,
    #[error("could not place container in a room")]
    ContainerPlacementError,
    #[error("could not place central square")]
    CentralSquarePlacementError,
    // #[error("at least one source or mineral not found")]
    // ResourceNotFound,
    #[error("unable to find available place for spawn!")]
    SpawnPlaceNotFound,
    #[error("unable to find a central square!")]
    CentralSquareNotFound,
    #[error("unable to determine a guide cell!")]
    GiudePointNotFound,
    #[error("room is unreachable")]
    UnreachableRoom,
    #[error("one of sources, the mineral or the controller is unreachable")]
    UnreachableResource,
    #[error("unable to find positions for all required structures")]
    StructurePlacementFailure,
    #[error("failed to plan road net")]
    RoadPlanFailure,
    #[error("failed to connect some points with roads")]
    RoadConnectionFailure,
    #[error("could not place ramparts to cover all of the interior of the base")]
    RampartPlacementFailure,
    #[error("could not place defencive perimeter")]
    PerimeterCreationFailed,
    #[error("could not create blueprint")]
    BlueprintCreationFailed,
    #[error("could not create room grid")]
    GridCreationFailed,
    #[error("farm is temporarly suspended")]
    FarmSuspended,
    #[error("room plan is already created")]
    AlreadyCreated, /* #[error("plan generation already finished")]
                     * PlanGenerationFinished, */
}

pub type CostedRoad = (RoomXY, usize);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RoomPlan {
    planned_cells: HashSet<PlannedCell>,
    built_lvl: u8,
}

impl RoomPlan {
    pub const fn new(planned_cells: HashSet<PlannedCell>) -> Self {
        Self { planned_cells, built_lvl: 0 }
    }

    pub const fn increment_lvl(&mut self) {
        if self.built_lvl < 8 {
            self.built_lvl += 1;
        }
    }

    pub fn is_occupied(&self, xy: RoomXY) -> bool {
        self.planned_cells.iter().any(|cell| cell.xy == xy)
    }

    pub const fn built_lvl(&self) -> u8 {
        self.built_lvl
    }

    pub fn find_by_xy(&self, xy: RoomXY) -> impl Iterator<Item = &PlannedCell> {
        self.planned_cells.iter().filter(move |c| c.xy == xy)
    }

    pub fn add_cell(&mut self, cell: PlannedCell) {
        self.planned_cells.insert(cell);
    }

    pub fn add_cells(&mut self, cells: impl Iterator<Item = PlannedCell>) {
        self.planned_cells.extend(cells);
    }

    pub fn replace_cell(&mut self, value: PlannedCell) {
        if let Some(mut cell) = self.planned_cells.take(&value) {
            cell.structure = value.structure;
            self.planned_cells.insert(cell);
        }
    }

    pub fn get_cell(&self, cell: PlannedCell) -> Option<&PlannedCell> {
        self.planned_cells.get(&cell)
    }

    pub fn pc_workplace(&self) -> Option<RoomXY> {
        self.planned_cells.iter().find_map(|cell| {
            if matches!(cell.structure, RoomStructure::Empty) { Some(cell.xy) } else { None }
        })
    }

    pub fn sender_xy(&self) -> Option<RoomXY> {
        self.planned_cells
            .iter()
            .find(|c| matches!(c.structure, RoomStructure::Link(LinkType::Sender)))
            .map(|c| c.xy)
    }

    pub fn receiver_xy(&self) -> Option<RoomXY> {
        self.planned_cells
            .iter()
            .find(|c| matches!(c.structure, RoomStructure::Link(LinkType::Receiver)))
            .map(|c| c.xy)
    }

    pub fn get_links(&self) -> impl Iterator<Item = &PlannedCell> {
        self.planned_cells.iter().filter(|c| matches!(c.structure, RoomStructure::Link(_)))
    }

    pub fn get_labs(&self) -> impl Iterator<Item = &PlannedCell> {
        self.planned_cells.iter().filter(|c| matches!(c.structure, RoomStructure::Lab(_)))
    }

    pub fn delete(&mut self, cell: PlannedCell) -> bool {
        self.planned_cells.remove(&cell)
    }

    pub fn current_lvl_buildings(&self) -> impl Iterator<Item = &PlannedCell> {
        self.planned_cells.iter().filter(|cell| {
            cell.b_lvl <= self.built_lvl
                && cell.r_lvl.is_none_or(|remove_lvl| remove_lvl > self.built_lvl)
        })
    }

    pub fn planned_cells(self) -> HashSet<PlannedCell> {
        self.planned_cells
    }

    pub fn occupied(&self) -> HashSet<RoomXY> {
        self.planned_cells.iter().map(|c| c.xy).collect()
    }

    pub fn storage(&self) -> Option<RoomXY> {
        self.planned_cells.iter().find_map(|c| match c.structure {
            RoomStructure::Storage => Some(c.xy),
            _ => None,
        })
    }

    pub fn partition_by_roads_or_not(&self) -> (HashSet<PlannedCell>, HashSet<PlannedCell>) {
        self.planned_cells
            .iter()
            .filter(|c| !matches!(c.structure, RoomStructure::Rampart(_)))
            .partition_map(|c| {
                if matches!(c.structure, RoomStructure::Road(_)) {
                    Either::Left(c)
                } else {
                    Either::Right(c)
                }
            })
    }

    pub fn roads(&self) -> HashSet<RoomXY> {
        self.planned_cells
            .iter()
            .filter(|c| matches!(c.structure, RoomStructure::Road(_)))
            .map(|c| c.xy)
            .collect()
    }

    pub fn containers(&self) -> HashSet<RoomXY> {
        self.planned_cells
            .iter()
            .filter(|c| matches!(c.structure, RoomStructure::Container(_)))
            .map(|c| c.xy)
            .collect()
    }

    pub fn perimeter(&self) -> HashSet<RoomXY> {
        self.planned_cells
            .iter()
            .filter(|c| matches!(c.structure, RoomStructure::Rampart(true)))
            .map(|c| c.xy)
            .collect()
    }

    pub fn costed_roads(&self, name: RoomName) -> HashMap<Position, usize> {
        self.planned_cells
            .iter()
            .filter_map(|c| match c.structure {
                RoomStructure::Road(distance) => {
                    Some((Position::new(c.xy.x, c.xy.y, name), distance))
                }
                _ => None,
            })
            .collect()
    }

    pub fn unwalkable_structures(&self) -> Vec<RoomXY> {
        self.planned_cells
            .iter()
            .filter(|c| !matches!(c.structure, RoomStructure::Road(_) | RoomStructure::Rampart(_)))
            .map(|c| c.xy)
            .collect()
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug, Default, PartialOrd, Ord)]
pub enum RoomPart {
    Green,  // internal safe zone
    Yellow, // internal cells range == 2 from perimeter (for towers + ramparts)
    Orange, //internal cells range == 1
    #[default]
    Red, // external
    Protected, // under rampart protection outside or inside the perimeter
    Wall,
    Structure,
    Road,
    Exit,
}

impl RoomPart {
    pub const fn is_internal(self) -> bool {
        matches!(self, RoomPart::Green | RoomPart::Yellow | RoomPart::Orange)
    }

    pub const fn is_partially_safe(self) -> bool {
        matches!(self, RoomPart::Yellow | RoomPart::Orange)
    }

    pub const fn is_safe(self) -> bool {
        matches!(self, RoomPart::Green | RoomPart::Protected)
    }

    pub const fn is_wall(self) -> bool {
        matches!(self, RoomPart::Wall)
    }

    pub const fn is_red(self) -> bool {
        matches!(self, RoomPart::Red)
    }

    pub const fn is_yellow(self) -> bool {
        matches!(self, RoomPart::Yellow)
    }
}

//don't use numeric representation because of mem::discriminant(self)
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum RoomStructure {
    Empty, //walkable cells near controller, reserved for power_creep and others??
    Spawn,
    Extension,
    Road(usize),
    Wall,
    Rampart(bool),
    Link(LinkType),
    Storage,
    Tower,
    Observer,
    PowerSpawn,
    Extractor,
    Lab(LabStatus),
    Terminal,
    Container(RoomPart),
    Nuker,
    Factory,
}

// A private, explicit ordering key (no serialization involved)
impl RoomStructure {
    #[inline]
    const fn build_order(&self) -> u8 {
        match *self {
            RoomStructure::Tower => 0,
            RoomStructure::Spawn => 1,
            RoomStructure::Storage => 2,
            RoomStructure::Extension => 3,
            RoomStructure::Terminal => 4,
            RoomStructure::Factory => 5,
            RoomStructure::Road(_) => 6,
            RoomStructure::Link(_) => 7,
            RoomStructure::Observer => 8,
            RoomStructure::PowerSpawn => 9,
            RoomStructure::Extractor => 10,
            RoomStructure::Lab(_) => 11,
            RoomStructure::Container(_) => 12,
            RoomStructure::Nuker => 13,
            RoomStructure::Wall => 14,
            RoomStructure::Rampart(_) => 15,
            RoomStructure::Empty => 16,
        }
    }
}

impl Eq for RoomStructure {}
impl PartialEq for RoomStructure {
    fn eq(&self, another: &RoomStructure) -> bool {
        //equality by variants only ignoring variant internals
        mem::discriminant(self) == mem::discriminant(another)
    }
}
impl Hash for RoomStructure {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //hash variant only ignoring variant internals
        mem::discriminant(self).hash(state);
    }
}

impl Ord for RoomStructure {
    fn cmp(&self, other: &Self) -> Ordering {
        self.build_order().cmp(&other.build_order())
    }
}
impl PartialOrd for RoomStructure {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<RoomStructure> for StructureType {
    type Error = &'static str;

    fn try_from(value: RoomStructure) -> Result<Self, Self::Error> {
        match value {
            RoomStructure::Spawn => Ok(StructureType::Spawn),
            RoomStructure::Extension => Ok(StructureType::Extension),
            RoomStructure::Road(_) => Ok(StructureType::Road),
            RoomStructure::Wall => Ok(StructureType::Wall),
            RoomStructure::Rampart(_) => Ok(StructureType::Rampart),
            RoomStructure::Link(_) => Ok(StructureType::Link),
            RoomStructure::Storage => Ok(StructureType::Storage),
            RoomStructure::Tower => Ok(StructureType::Tower),
            RoomStructure::Observer => Ok(StructureType::Observer),
            RoomStructure::PowerSpawn => Ok(StructureType::PowerSpawn),
            RoomStructure::Extractor => Ok(StructureType::Extractor),
            RoomStructure::Lab(_) => Ok(StructureType::Lab),
            RoomStructure::Terminal => Ok(StructureType::Terminal),
            RoomStructure::Container(_) => Ok(StructureType::Container),
            RoomStructure::Nuker => Ok(StructureType::Nuker),
            RoomStructure::Factory => Ok(StructureType::Factory),
            RoomStructure::Empty => Err("unmapped structure!"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub enum LabStatus {
    Boost(ResourceType),
    Input,
    #[default]
    Output,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum LinkType {
    Sender,
    Receiver,
    Ctrl,
    Source,
}

// Information about structure present in a cell
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct PlannedCell {
    pub xy: RoomXY,
    pub structure: RoomStructure,
    pub b_lvl: u8,
    pub r_lvl: Option<u8>,
}

impl PlannedCell {
    pub const fn new(xy: RoomXY, structure: RoomStructure, b_lvl: u8, r_lvl: Option<u8>) -> Self {
        Self { xy, structure, b_lvl, r_lvl }
    }

    pub const fn searchable(xy: RoomXY, structure: RoomStructure) -> Self {
        Self { xy, structure, b_lvl: 0, r_lvl: None }
    }
}

impl Eq for PlannedCell {}
impl PartialEq for PlannedCell {
    fn eq(&self, another: &PlannedCell) -> bool {
        self.xy == another.xy && self.structure == another.structure
    }
}
impl Hash for PlannedCell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.xy.hash(state);
        self.structure.hash(state);
    }
}

pub trait TerrainSource {
    fn terrain_at(&self, x: u8, y: u8) -> Terrain;
}
impl<F> TerrainSource for F
where
    F: Fn(u8, u8) -> Terrain,
{
    #[inline]
    fn terrain_at(&self, x: u8, y: u8) -> Terrain {
        (self)(x, y)
    }
}

/* ---------------- helpers ---------------- */

pub fn build_wall_bitmap<T: TerrainSource>(src: &T) -> Walls {
    let mut m = [[false; ROOM_SIZE as usize]; ROOM_SIZE as usize];
    for y in 0..ROOM_SIZE {
        for x in 0..ROOM_SIZE {
            m[y as usize][x as usize] = src.terrain_at(x, y) == Terrain::Wall;
        }
    }
    m
}

#[inline]
const fn is_wall(walls: &Walls, p: RoomXY) -> bool {
    walls[p.y.u8() as usize][p.x.u8() as usize]
}

// #[cfg(test)]
// mod tests {
//     use std::{str::FromStr, vec};
//     use screeps::{Position, RoomCoordinate, RoomName, RoomXY};

//     use crate::rooms::constructions::{blueprints::Blueprint, RoomPlan};

//     use super::Walls;

//     #[test]
//     fn plan_room_test() {
//         let ctrl = ctrl();
//         let mineral = mineral().xy();
//         let initial_spawn = spawn();
//         let sources = sources();

//         let blueprint = Blueprint::new( room_name(), ctrl, initial_spawn,
// sources, mineral, WALLS)             .expect("expect room plan");

//         // let expected_config = RoadConfig::new(Dash::new(Turn::Second,
// Turn::Second), false);         // let expected_cross_road = unsafe {
// RoomXY::unchecked_new(22, 26) };

//         // assert_eq!(expected_config, *blueprint.config(), "incorrect
// config");         // assert_eq!(expected_cross_road, *blueprint.cross_road(),
// "incorrect crossroad");         // assert_eq!((13, 17, 31, 36),
// blueprint.rectangle(), "incorrect rectangle!");         // assert_eq!(2500,
// blueprint.grid().len(), "incorrect grid!");

//         let room_plan: RoomPlan = blueprint.try_into().expect("expect room
// plan!");         // room_plan.planned_cells().iter()
//             // .sorted_by(|xy1, xy2| match Ord::cmp(&xy1.xy.y.u8(),
// &xy2.xy.y.u8()) {             //     Ordering::Equal =>
// Ord::cmp(&xy1.xy.x.u8(), &xy2.xy.x.u8()),             //
// Ordering::Greater => Ordering::Greater,             //     Ordering::Less =>
// Ordering::Less             // })
//         //     .for_each(|cell| println!("c: {:?}", cell));
//         println!("len: {}", room_plan.planned_cells().len());

//         let tower = unsafe { RoomXY::unchecked_new(15, 19) };
//         assert!(room_plan.planned_cells().iter().find(|c| c.xy ==
// tower).is_some(), "expect tower at (15,19)");

//         assert_eq!(213, room_plan.planned_cells().len(), "invalid planned
// cells len!")     }

//     #[test]
//     fn room_grid_test() {
//         let spawn = spawn();
//         let sources = sources();
//         let perimeter = smallest_perimeter(spawn, &sources,
// &WALLS).expect("expect perimeter");

//         let grid = room_grid(&perimeter, &WALLS).expect("expect grid");

//         let exit = unsafe { RoomXY::unchecked_new(0, 30) };
//         let wall = unsafe { RoomXY::unchecked_new(24, 30) };
//         let red = unsafe { RoomXY::unchecked_new(15, 35) };
//         let orange = unsafe { RoomXY::unchecked_new(23, 35) };
//         let yellow = unsafe { RoomXY::unchecked_new(23, 34) };
//         let green = unsafe { RoomXY::unchecked_new(23, 33) };
//         let ctrl = unsafe { RoomXY::unchecked_new(34, 14) };
//         let source = unsafe { RoomXY::unchecked_new(15, 27) };

//         assert_eq!(grid.get(&exit), Some(&RoomPart::Exit), "expect exit
// part!");         assert_eq!(grid.get(&wall), Some(&RoomPart::Wall), "expect
// wall part!");         assert_eq!(grid.get(&red), Some(&RoomPart::Red),
// "expect red part!");         assert_eq!(grid.get(&orange),
// Some(&RoomPart::Orange), "expect red part!");         assert_eq!(grid.get(&
// yellow), Some(&RoomPart::Yellow), "expect yellow part!");         assert_eq!
// (grid.get(&green), Some(&RoomPart::Green), "expect green part!");
//         assert_eq!(grid.get(&ctrl), Some(&RoomPart::Wall), "expect wall
// part!");         assert_eq!(grid.get(&source), Some(&RoomPart::Wall), "expect
// wall part!");     }

//     pub fn perimeter(spawn: Option<RoomXY>, sources: &[RoomXY]) -> Perimeter
// {         smallest_perimeter(spawn, &sources, &WALLS).expect("expect
// perimeter")     }

//     pub fn grid(perimeter: &Perimeter) -> HashMap<RoomXY, RoomPart> {
//         room_grid(&perimeter, &WALLS).expect("expect grid")
//     }

//     pub fn room_name() -> RoomName {
//         unsafe { RoomName::from("W12N9") }
//     }

//     pub fn sources() -> Vec<RoomXY> {
//         unsafe { vec![ RoomXY::unchecked_new(15, 17),
// RoomXY::unchecked_new(15, 27) ] }     }

//     pub fn ctrl() -> RoomXY {
//         unsafe { RoomXY::unchecked_new(34, 14) }
//     }

//     pub fn mineral() -> Position {
//         Position::new(RoomCoordinate(11), RoomCoordinate(11),
// RoomName::from_str("W12N9").expect("expect room name"))     }

//     pub fn spawn() -> Option<RoomXY> {
//         unsafe { Some(RoomXY::unchecked_new(18, 25)) }
//     }

//     pub const WALLS: Walls = [[true, true, true, true, true, true, true,
// true, true, true, true, true, true, true, true, true, true, true, true, true,
// true, true, true, true, false, false, false, false, false, false, false,
// false, false, false, false, false, true, true, true, true, true, true, true,
// true, true, true, true, true, true, true], [true, true, true, true, false,
// false, false, false, true, true, true, false, false, false, true, true, true,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, true, true, true,
// true, true, true, true, true, true, true, true, true, true, true], [true,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, false, true, true, true, true, false, false, false, false,
// false, false, true, false, false, false, false, false, false, false, false,
// false, true, true, true, true, true, true, true, true, true, true, true,
// true, true, true, true], [true, true, true, true, true, true, false, false,
// false, false, false, false, false, false, false, false, true, true, false,
// false, false, false, false, false, true, true, true, false, false, false,
// false, false, false, false, false, true, true, true, true, true, true, true,
// true, true, true, true, false, false, true, true], [true, true, true, true,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, true,
// true, false, false, false, false, false, false, false, false, true, true,
// true, true, true, true, true, true, true, true, false, false, false, true,
// true], [false, false, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, true,
// true, true, true, true, true, true, true, true, true, true, false, false,
// false, false, false, false, true, true], [false, false, false, true, true,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, true, true, true, true, true, true, true, true,
// true, true, false, false, false, false, false, false, false, true, true],
// [false, false, false, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, true, true,
// true, true, true, true, true, true, true, true, false, false, false, false,
// false, false, false, true, true], [false, false, true, true, true, true,
// true, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, true, true, true,
// true, false, false, false, false, false, false, false, false, false, true],
// [false, false, false, true, true, true, false, false, false, false, false,
// false, true, true, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false], [false, false, false, false,
// false, false, false, false, false, false, false, true, true, true, true,
// false, false, false, false, false, false, false, false, false, false, false,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false], [false, false, false, false, false, false, false, false,
// false, false, false, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, true, true, true, true, true,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false], [false,
// false, false, false, false, false, false, false, false, false, false, false,
// true, true, true, true, true, false, false, false, false, false, false,
// false, true, true, true, true, true, true, true, true, true, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false], [false, false, false, false, false, false,
// false, false, false, false, false, false, false, true, true, true, true,
// false, false, false, false, false, false, false, true, true, true, true,
// true, true, true, true, true, true, false, false, false, false, false, true,
// false, false, false, false, false, false, false, false, false, false],
// [false, false, false, false, false, false, false, false, false, false, false,
// false, false, true, true, true, false, false, false, false, false, false,
// false, false, true, true, true, true, true, true, true, true, true, true,
// true, false, false, false, true, true, true, false, false, false, false,
// false, false, false, false, false], [false, false, false, false, false,
// false, false, false, false, true, true, false, true, true, true, true, false,
// false, false, false, false, false, false, false, true, true, true, true,
// true, true, true, true, true, true, true, false, false, false, true, true,
// true, true, false, false, false, false, false, false, false, false], [false,
// false, true, true, false, false, false, false, true, true, true, true, true,
// true, true, true, false, false, false, false, false, false, false, false,
// true, true, true, true, true, true, true, true, true, true, true, false,
// false, false, true, true, true, true, false, false, false, false, false,
// false, false, false], [true, true, true, false, false, false, false, false,
// true, true, true, true, true, true, true, true, true, false, false, false,
// false, false, false, true, true, true, true, true, true, true, false, false,
// false, false, false, false, false, false, false, true, true, true, false,
// false, false, false, false, false, false, false], [true, true, true, false,
// false, false, false, false, true, true, true, true, true, true, true, true,
// false, false, false, false, false, false, false, true, true, true, true,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, true, true, false, false, false, false, false, false, false,
// false], [true, true, false, false, false, false, false, false, false, true,
// true, false, false, false, false, false, false, false, false, false, false,
// false, false, true, true, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false], [true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, true, true, true, true,
// true, true, true, true, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false], [true, false, false, false, false, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, true, true, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, true, true, true, false, false], [true, false, false, false, false,
// true, true, false, false, false, false, false, false, true, true, true,
// false, false, false, false, false, false, false, false, true, true, true,
// true, false, false, false, false, false, false, false, false, true, true,
// false, false, false, false, false, false, false, true, true, true, false,
// false], [true, false, false, false, false, true, true, false, false, false,
// false, false, false, true, true, true, true, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, true, true, true, true, true, false, false, false, false,
// false, false, true, true, true, false, false], [true, false, false, false,
// true, true, false, false, false, false, false, false, false, true, true,
// true, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, true, true, true, true, true, true, true, true,
// true, false, false, false, false, false, false, true, true, true, false,
// false], [true, false, false, true, true, true, false, false, false, false,
// false, false, true, true, true, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, true, true, true,
// true, true, true, true, true, true, true, false, false, false, false, false,
// true, true, true, false, false, false], [true, false, false, true, true,
// true, true, false, false, false, false, false, true, true, true, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, true, true, true, true, true, true, true, true, true, true, true,
// true, false, false, false, false, true, true, true, false, false, false],
// [false, false, true, true, true, true, true, true, false, false, false,
// false, true, true, true, true, false, false, false, false, false, false,
// false, false, false, false, false, false, true, true, true, true, true, true,
// true, true, true, true, true, true, false, false, false, true, true, true,
// false, false, false, true], [false, false, true, true, true, true, true,
// true, true, false, false, false, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, true,
// true, true, true, true, true, true, true, true, true, false, false, false,
// false, true, true, true, false, false, false, true], [false, false, true,
// true, true, true, true, true, true, true, false, false, true, true, true,
// true, false, false, false, false, false, false, false, false, true, false,
// false, false, false, false, true, true, true, true, true, true, true, true,
// false, false, false, false, false, true, true, false, false, false, false,
// true], [false, false, false, true, true, true, true, true, true, false,
// false, false, true, true, true, true, false, false, false, false, false,
// false, false, true, true, true, false, false, false, false, true, true, true,
// true, true, true, true, false, false, false, false, false, true, true, true,
// false, false, false, false, true], [false, false, false, false, false, false,
// false, false, false, false, false, false, false, true, true, true, true,
// false, false, false, false, false, false, false, true, false, false, false,
// false, false, false, true, true, true, true, true, true, false, false, false,
// false, true, true, true, true, true, false, false, false, true], [true,
// false, false, false, false, false, false, false, false, false, false, false,
// false, true, true, true, true, true, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, true, true,
// true, true, true, true, false, false, true, true, true, true, true, true,
// true, false, false, true], [true, true, false, false, true, true, true, true,
// true, false, false, false, false, false, true, true, true, true, true, true,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, true, true, true, false, false, true, true, true,
// true, true, true, true, false, false, true], [true, true, true, true, true,
// true, true, true, true, true, false, false, false, false, false, true, true,
// true, true, true, true, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, true, true, false,
// false, false, true, true, true, true, true, false, false, false, true],
// [true, true, true, true, true, true, true, true, true, true, false, false,
// false, false, false, false, false, false, true, true, true, true, false,
// false, false, false, false, false, true, false, false, false, false, false,
// false, false, true, true, false, false, false, false, false, false, false,
// false, false, false, false, true], [true, true, true, true, true, true, true,
// true, true, false, false, false, false, false, false, false, false, false,
// false, true, true, false, false, false, false, false, false, true, true,
// true, true, false, false, false, false, true, true, true, true, false, false,
// false, false, false, false, false, false, false, false, true], [true, true,
// true, true, true, true, true, true, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, true, true, true, true, false, false, false, true, true,
// true, true, false, false, false, false, false, false, false, false, false,
// false, true], [true, true, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, true, true, true,
// false, false, false, true, true, true, true, false, false, false, false,
// false, true, true, true, false, false, false], [true, true, true, true, true,
// true, true, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, true, true, false, false,
// false, false, false, true, true, true, false, false, false, true, true,
// false, false, false, false, false, false, true, true, true, false, false,
// false], [true, true, false, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, true, true, true, true, false, false, false, false, true, true, true,
// false, false, false, false, false, false, false, false, false, false, false,
// true, true, true, true, false, false], [true, false, false, false, true,
// true, true, false, false, false, false, false, false, false, true, true,
// false, false, false, false, false, true, true, true, true, true, false,
// false, false, false, false, true, false, false, false, false, false, false,
// false, false, false, false, false, false, true, true, true, true, false,
// false], [true, false, false, false, false, false, false, false, false, false,
// false, false, false, true, true, true, true, false, false, false, true, true,
// true, true, true, true, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, true, true, true, false, false], [true, false, false, false, false,
// false, false, false, false, false, false, false, true, true, true, true,
// true, false, false, false, true, true, true, true, true, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, true, true, true, true],
// [true, false, false, false, false, false, false, false, false, false, false,
// false, true, true, true, true, false, false, false, false, false, true, true,
// true, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, true, true, true], [true, true, false, false, false, false,
// false, false, false, false, false, false, true, true, true, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, true, true, true],
// [true, true, true, false, false, false, false, false, false, false, false,
// false, false, true, true, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, false,
// false, false, false, false, true, true], [true, true, true, true, false,
// false, false, false, false, false, false, false, false, true, true, false,
// false, false, false, false, false, false, false, false, false, false, false,
// true, true, true, false, false, false, false, false, false, false, false,
// false, false, false, false, false, false, false, false, false, false, true,
// true], [true, true, true, true, true, true, true, true, true, true, false,
// false, true, true, true, true, false, false, true, true, true, false, false,
// false, false, false, true, true, true, true, true, false, false, false, true,
// true, false, false, false, false, false, false, true, true, true, false,
// false, true, true, true], [true, true, true, true, true, true, true, true,
// true, true, true, true, true, true, true, true, true, true, true, true, true,
// true, true, true, true, true, true, true, true, true, true, true, true, true,
// true, true, true, true, true, true, true, true, true, true, true, true, true,
// true, true, true]]; }
