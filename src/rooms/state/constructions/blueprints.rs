// use log::info;
// use screeps::{find, game, pathfinder::{MultiRoomCostResult, SearchGoal, SearchOptions, SearchResults, SingleRoomCostResult}, CostMatrix, Direction, FindPathOptions, HasPosition, Path, Position, Room, RoomCoordinate, RoomName, RoomPosition, RoomXY, Step, StructureObject};
// use smallvec::SmallVec;
// use std::{collections::{HashMap, HashSet}, cmp::Ordering::{self, *}, error::Error, u32};
// use itertools::Itertools;
// use crate::{movement::{callback::{construction_callback, SingleRoomCallback}, Movement, MovementGoalBuilder}, rooms::constructions::{build_wall_bitmap, RoomPlan, RoomPlanner}, utils::constants::MAX_OPS};

// // use self::{
// //     polygon::{smallest_perimeter, Perimeter},
// //     central::{CentralSquare, central_square},
// //     spawns::spawn_space,
// //     roads::{RoadConfig, RoadNet, best_net}
// // };
// use super::{
//     RoomPlannerError, RoomPart, OuterRectangle, PlannedCell, Walls, is_wall, RoomStructure,
//     xy_util::{ROOM_SIZE, get_sides, counter_clockwise_dir, outside_rect}};
// use self::owned::{CentralSquare, RoadConfig, Perimeter};

// // mod owned;
// mod remoted;
// mod source_keeper;

// type Sat = [[u16; ROOM_SIZE]; ROOM_SIZE];
// type Route = (Position, Vec<Step>);

// #[derive(Debug)]
// pub struct RemotedBlueprint {
//     room_name: RoomName,
//     structures: HashMap<RoomName, Vec<RoomXY>>,
//     storage: Position,
//     sources: Vec<Position>,
// }

// #[derive(Debug)]
// pub struct SourceKeeperBlueprint {
//     room_name: RoomName,
//     structures: HashMap<RoomName, Vec<RoomXY>>,
//     storage: Position,
//     mineral: Position,
//     sources: Vec<Position>,
//     grid: HashMap<RoomXY, RoomPart>
// }

// #[derive(Debug)]
// pub struct OwnedBlueprint {
//     // room_name: RoomName,
//     sources: Vec<RoomXY>,
//     ctrl: RoomXY,
//     perimeter: Perimeter,
//     config: RoadConfig,
//     grid: HashMap<RoomXY, RoomPart>,
//     central: CentralSquare,
//     spawns: Vec<RoomXY>,
//     mineral: Position,
// }

// fn cmp_routes(r1: &Route, r2: &Route, guides: &[RoomXY]) -> Ordering {
//     match r1.1.len().cmp(&r2.1.len()) {
//         Equal => {
//             let a = last_step_min_range_to_guides(r1, guides);
//             let b = last_step_min_range_to_guides(r2, guides);
//             match (a, b) {
//                 (Some(x), Some(y)) => x.cmp(&y),
//                 (Some(_), None)    => Greater, // r2 (None) preferred
//                 (None, Some(_))    => Less,    // r1 (None) preferred
//                 (None, None)       => Equal,
//             }
//         }
//         other => other,
//     }
// }

// fn last_step_min_range_to_guides(route: &Route, guides: &[RoomXY]) -> Option<u32> {
//     let last = route.1.last()?;
//     let xy = RoomXY::new(RoomCoordinate(last.x as u8), RoomCoordinate(last.y as u8));
//     guides
//         .iter()
//         .map(|edge| edge.get_range_to(xy))
//         .min()
// }

// // fn best_path() -> Option<(Position, Vec<Step>)> {
// //     roads.iter()
// //         .flat_map(|to| {
// //             targets.clone().into_iter()
// //                 .map(|target| {
// //                     (target, find_in_room_path(
// //                         target.into(),
// //                         *to,
// //                         1,
// //                         construction_sk_callback(&self.grid, &HashSet::new())))
// //                 })
// //         })
// //         .sorted_by(|(_, path1), (_, path2)| match Ord::cmp(&path1.len(), &path2.len()) {
// //             Ordering::Equal => {
// //                 if let (Some(p1), Some(p2)) = (path1.last(), path2.last()) {
// //                     let xy1 = RoomXY::new(RoomCoordinate(p1.x as u8), RoomCoordinate(p1.y as u8));
// //                     let xy2 = RoomXY::new(RoomCoordinate(p2.x as u8), RoomCoordinate(p2.y as u8));

// //                     let range1 = self.guides.iter()
// //                         .map(|edge| edge.get_range_to(xy1))
// //                         .sorted()
// //                         .next().unwrap_or_default();
// //                     let range2 = self.guides.iter()
// //                         .map(|edge| edge.get_range_to(xy2))
// //                         .sorted()
// //                         .next().unwrap_or_default();

// //                     Ord::cmp(&range1, &range2)
// //                 } else if path1.last().is_some() {
// //                     Ordering::Greater
// //                 } else {
// //                     Ordering::Less
// //                 }
// //             },
// //             Ordering::Greater => Ordering::Greater,
// //             Ordering::Less => Ordering::Less
// //         })
// //         .next()
// // }

// // pub fn find_in_room_path3(
// //     from: RoomPosition,
// //     to: RoomXY,
// //     range: u32,
// //     grid: &HashMap<RoomXY, RoomPart>,
// //     containers: &HashSet<RoomXY>) -> Vec<Step>
// // {
// //     let fpo = FindPathOptions::<SingleRoomCallback, SingleRoomCostResult>::new()
// //         .cost_callback(construction_callback(grid, containers))
// //         .range(range)
// //         .ignore_creeps(true)
// //         .plain_cost(1)
// //         .swamp_cost(2);

// //     match from.find_path_to_xy(to.x, to.y, Some(fpo)) {
// //         Path::Vectorized(v) => v,
// //         Path::Serialized(_) => Vec::new() //todo deserialize
// //     }
// // }

// // pub fn find_in_room_path2(
// //     from: RoomPosition,
// //     to: RoomXY,
// //     range: u32,
// //     grid: &HashMap<RoomXY, RoomPart>) -> Vec<Step>
// // {
// //     let fpo = FindPathOptions::<SingleRoomCallback, SingleRoomCostResult>::new()
// //         .cost_callback(construction_callback2(grid))
// //         .range(range)
// //         .ignore_creeps(true)
// //         .plain_cost(1)
// //         .swamp_cost(2);

// //     match from.find_path_to_xy(to.x, to.y, Some(fpo)) {
// //         Path::Vectorized(v) => v,
// //         Path::Serialized(_) => Vec::new() //todo deserialize
// //     }
// // }
// // Some(SearchOptions::new(callback::prefer_swamp_callback(options))
// //                     .max_ops(MAX_OPS)
// //                     .max_rooms(max_rooms)
// //                     .swamp_cost(1)
// //                     .flee(flee)
// //                     .heuristic_weight(HEURISTIC_WEIGHT)
// //                 )
// // fn find_paths<C>(
// //     from: Position,
// //     to: impl Iterator<Item = SearchGoal>,
// //     callback: C
// // ) -> SearchResults
// //     where C: FnMut(RoomName) -> MultiRoomCostResult
// // {
// //     let options = SearchOptions::new(callback)
// //         .max_ops(2000) //default value, could be reduced
// //         .max_rooms(16) //default value, could be reduced
// //         .plain_cost(1)
// //         .swamp_cost(2);

// //     screeps::pathfinder::search_many(from, to, Some(options))
// // }

// fn find_in_room_path<C>(
//     from: RoomPosition,
//     to: RoomXY,
//     range: u32,
//     callback: C
// ) -> Vec<Step>
//     where C: FnMut(RoomName, CostMatrix) -> SingleRoomCostResult
// {
//     let fpo = FindPathOptions::<SingleRoomCallback, SingleRoomCostResult>::new()
//         .cost_callback(callback)
//         .range(range)
//         .ignore_creeps(true)
//         .plain_cost(1)
//         .swamp_cost(2);

//     match from.find_path_to_xy(to.x, to.y, Some(fpo)) {
//         Path::Vectorized(v) => v,
//         Path::Serialized(_) => Vec::new() //todo deserialize
//     }
// }
// // fn find_path(from: Position, to: Position, range: u32, grid: &HashMap<RoomXY, RoomPart>) -> Vec<Position> {
// //     // let options = Some(SearchOptions::new(callback::construction_callback()).swamp_cost(1));
// //     let options = if grid.is_empty() {
// //         Some(SearchOptions::new(callback::construction_callback()).swamp_cost(1))
// //     } else {
// //         Some(SearchOptions::new(callback::construction_with_grid_callback(grid)).swamp_cost(1))
// //     };

// //     screeps::pathfinder::search(
// //         from,
// //         to,
// //         range,
// //         options).path()
// // }

// // fn find_path(from: Position, to: Position, range: u32, grid: &HashMap<RoomXY, RoomPart>) -> Vec<Position> {
// //     // let options = Some(SearchOptions::new(callback::construction_callback()).swamp_cost(1));
// //     let options = if grid.is_empty() {
// //         Some(SearchOptions::new(callback::construction_callback()).swamp_cost(1))
// //     } else {
// //         Some(SearchOptions::new(callback::construction_with_grid_callback(grid)).swamp_cost(1))
// //     };

// //     screeps::pathfinder::search(
// //         from,
// //         to,
// //         range,
// //         options).path()
// // }


// #[cfg(test)]
// mod tests {
//     use crate::rooms::constructions::tests::{sources, spawn, WALLS};
//     use crate::rooms::constructions::blueprints::smallest_perimeter;
//     use super::*;
//     use screeps::local::RoomXY;

//     #[test]
//     fn room_grid_test() {
//         let spawn = spawn();
//         let sources = sources();
//         let perimeter = smallest_perimeter(spawn, &sources, &WALLS).expect("expect perimeter");

//         let grid = room_grid(&perimeter, &WALLS).expect("expect grid");

//         let exit = unsafe { RoomXY::unchecked_new(0, 30) };
//         let wall = unsafe { RoomXY::unchecked_new(24, 30) };
//         let red = unsafe { RoomXY::unchecked_new(15, 35) };
//         let orange = unsafe { RoomXY::unchecked_new(23, 35) };
//         let yellow = unsafe { RoomXY::unchecked_new(23, 34) };
//         let green = unsafe { RoomXY::unchecked_new(23, 33) };
//         let ctrl = unsafe { RoomXY::unchecked_new(34, 14) };
//         let source = unsafe { RoomXY::unchecked_new(15, 27) };

//         assert_eq!(grid.get(&exit), Some(&RoomPart::Exit), "expect exit part!");
//         assert_eq!(grid.get(&wall), Some(&RoomPart::Wall), "expect wall part!");
//         assert_eq!(grid.get(&red), Some(&RoomPart::Red), "expect red part!");
//         assert_eq!(grid.get(&orange), Some(&RoomPart::Orange), "expect red part!");
//         assert_eq!(grid.get(&yellow), Some(&RoomPart::Yellow), "expect yellow part!");
//         assert_eq!(grid.get(&green), Some(&RoomPart::Green), "expect green part!");
//         assert_eq!(grid.get(&ctrl), Some(&RoomPart::Wall), "expect wall part!");
//         assert_eq!(grid.get(&source), Some(&RoomPart::Wall), "expect wall part!");
//     }

//     pub fn perimeter(spawn: Option<RoomXY>, sources: &[RoomXY]) -> Perimeter {
//         smallest_perimeter(spawn, &sources, &WALLS).expect("expect perimeter")
//     }

//     pub fn grid(perimeter: &Perimeter) -> HashMap<RoomXY, RoomPart> {
//         room_grid(&perimeter, &WALLS).expect("expect grid")
//     }
// }