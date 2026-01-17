use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::iter::once;

use itertools::Itertools;
use screeps::{
    Direction, HasPosition, OutOfBoundsError, Position, RoomCoordinate, RoomPosition, RoomXY, Step,
};

use self::central::central_square;
use self::polygon::{smallest_perimeter, walk_border};
use self::roads::{Dash, RoadNet, Turn, Variable, best_net};
use self::spawns::spawn_space;
use super::xy_util::{outside_rect, square_sides};
use super::{
    OuterRectangle, RoomPart, RoomPlan, RoomPlannerError, Walls, build_wall_bitmap, is_wall,
};
use crate::movement::{callback::closest_in_room_range, find_path_in_room};
use crate::rooms::wrappers::claimed::Claimed;

mod central;
mod controller;
mod extensions;
mod mineral;
mod observer;
mod polygon;
mod ramparts;
mod roads;
mod sources;
mod spawns;
mod towers;

type Route = (RoomXY, Vec<Step>);

impl Claimed {
    pub fn generate_plan(
        &self,
        rect: Option<OuterRectangle>,
    ) -> Result<RoomPlan, RoomPlannerError> {
        let sources: Vec<RoomXY> = self.sources.iter().map(|source| source.pos().xy()).collect();

        let ctrl = self.controller.pos().xy();

        let initial_spawn = self.spawns.first().map(|spawn| spawn.pos().xy());

        let mineral = self.mineral.pos().xy();

        let terrain = self.room.get_terrain();
        let walls = build_wall_bitmap(&|a, b| terrain.get(a, b));

        let perimeter = match rect {
            Some(r) => Perimeter::new(r, &walls),
            None => smallest_perimeter(initial_spawn, &sources, &walls)?,
        };

        let grid = room_grid(&perimeter, &walls)?;

        let RoadNet { config, roads, mut squares } =
            best_net(perimeter.rectangle(), initial_spawn, &grid)?;

        // todo guide cell could be an exit from farm to a base
        // the idea is to turn the base in direction to the guide cell!
        let guide = guide_cell(ctrl, &sources, perimeter.rectangle())?;

        let central = central_square(guide, initial_spawn, &roads, &mut squares, &walls)?;
        let spawns = spawn_space(&central, initial_spawn, &squares, &walls)?;

        let mut plan = RoomPlan::new(central.plan()?);
        let storage = plan
            .storage()
            .map(|xy| Position::new(xy.x, xy.y, self.get_name()))
            .ok_or(RoomPlannerError::StorageNotFound)?;

        spawns::plan(&spawns, &mut plan);
        ramparts::plan(&perimeter, &mut plan);
        towers::plan(&perimeter, &grid, &mut plan);
        config.plan(perimeter.rectangle(), &grid, &mut plan);
        sources::plan(storage, &sources, &grid, &mut plan)?;
        controller::plan(storage, ctrl, &grid, &mut plan)?;
        mineral::plan(storage, mineral, &grid, &mut plan)?;
        extensions::plan(storage.xy(), &grid, &mut plan);
        observer::plan(storage.xy(), &grid, &mut plan);

        Ok(plan)
    }

    // pub fn generate_plan(&self) -> Result<RoomPlan, RoomPlannerError> {
    //     let mut cells = HashSet::new();

    //     let storage = self.storage().expect("expect storage");

    //     cells.insert(PlannedCell::new(storage.pos().xy(), RoomStructure::Storage,
    // 4, None));     cells.extend(self.power_spawn.as_ref()
    //         .map(|p| PlannedCell::new(p.pos().xy(), RoomStructure::PowerSpawn, 8,
    // None)));     cells.extend(self.observer.as_ref()
    //         .map(|p| PlannedCell::new(p.pos().xy(), RoomStructure::Observer, 8,
    // None)));     cells.extend(self.factory.as_ref()
    //         .map(|p| PlannedCell::new(p.pos().xy(), RoomStructure::Factory, 7,
    // None)));     cells.extend(self.terminal.as_ref()
    //         .map(|p| PlannedCell::new(p.pos().xy(), RoomStructure::Terminal, 6,
    // None)));     cells.extend(self.nuker.as_ref()
    //         .map(|p| PlannedCell::new(p.pos().xy(), RoomStructure::Nuker, 8,
    // None)));     cells.insert(PlannedCell::new(self.mineral.pos().xy(),
    // RoomStructure::Extractor, 6, None));     cells.extend(self.ramparts.
    // iter()         .map(|r| PlannedCell::new(r.pos().xy(),
    // RoomStructure::Rampart(true), 5, None)));     cells.extend(self.spawns.
    // iter()         .map(|r| PlannedCell::new(r.pos().xy(),
    // RoomStructure::Spawn, 8, None)));     cells.extend(self.extensions.iter()
    //         .map(|r| PlannedCell::new(r.pos().xy(), RoomStructure::Extension, 8,
    // None)));     cells.extend(self.containers.iter()
    //         .map(|r| PlannedCell::new(r.pos().xy(),
    // RoomStructure::Container(RoomPart::Red), 8, None)));
    //     cells.extend(self.towers.iter()
    //         .map(|r| PlannedCell::new(r.pos().xy(), RoomStructure::Tower, 8,
    // None)));     cells.extend(self.roads.iter()
    //         .map(|r| PlannedCell::new(r.pos().xy(), RoomStructure::Road(1), 2,
    // None)));     cells.extend(self.links.iter()
    //         .map(|r| PlannedCell::new(r.pos().xy(),
    // RoomStructure::Link(super::LinkType::Source), 8, None)));
    //     cells.extend(self.labs.iter()
    //         .map(|r| PlannedCell::new(r.pos().xy(),
    // RoomStructure::Lab(LabStatus::Output), 8, None)));

    //     Ok(RoomPlan { planned_cells: cells, built_lvl: 8 })
    // }
}

fn room_grid(
    perimeter: &Perimeter,
    walls: &Walls,
) -> Result<HashMap<RoomXY, RoomPart>, RoomPlannerError> {
    let mut grid = HashMap::new();

    let (x0, y0, x1, y1) = perimeter.rectangle();
    let border = perimeter.full_path_sorted();

    for y in 0..screeps::ROOM_SIZE {
        for x in 0..screeps::ROOM_SIZE {
            let xy = unsafe { RoomXY::unchecked_new(x, y) };

            if is_wall(walls, xy) {
                grid.insert(xy, RoomPart::Wall);
            } else if xy.is_room_edge() {
                grid.insert(xy, RoomPart::Exit);
            } else if perimeter.ramparts().contains(&xy) {
                grid.insert(xy, RoomPart::Protected);
            } else if x > x0 && x < x1 && y > y0 && y < y1 {
                let x_line: Vec<_> = border.iter().filter(|xy| xy.y.u8() == y).collect();

                let first_x = x_line.first().ok_or(RoomPlannerError::GridCreationFailed)?;
                let last_x = x_line.last().ok_or(RoomPlannerError::GridCreationFailed)?;

                if x < first_x.x.u8() || x > last_x.x.u8() {
                    grid.insert(xy, RoomPart::Red);
                } else {
                    // xy definatelly inside the perimeter?
                    let range = border
                        .iter()
                        .map(|edge| xy.get_range_to(*edge))
                        .min()
                        .ok_or(RoomPlannerError::GridCreationFailed)?;

                    let part = match range {
                        0 => Err(RoomPlannerError::GridCreationFailed),
                        1 => Ok(RoomPart::Orange),
                        2 => Ok(RoomPart::Yellow),
                        _ => Ok(RoomPart::Green),
                    }?;
                    grid.insert(xy, part);
                }
            } else {
                grid.insert(xy, RoomPart::Red);
            }
        }
    }

    Ok(grid)
}

fn place_for_container(
    storage: Position,
    target: RoomXY,
    roads: HashSet<RoomXY>,
    unwalkable: HashSet<RoomXY>,
    grid: &HashMap<RoomXY, RoomPart>,
) -> Option<RoomXY> {
    let (road_cells, empty_cells): (Vec<RoomXY>, Vec<_>) = walkable_neighbors(target, grid)
        .filter(|xy| !unwalkable.contains(xy))
        .partition(|xy| roads.contains(xy));

    empty_cells
        .into_iter()
        .map(|xy| walkable_range(storage.into(), xy, grid))
        .min_by(|r1, r2| cmp_routes(r1, r2, storage))
        .or_else(|| {
            road_cells
                .into_iter()
                .map(|xy| walkable_range(storage.into(), xy, grid))
                .min_by(|r1, r2| cmp_routes(r1, r2, storage))
        })
        .map(|(xy, _)| xy)
}

fn walkable_range(from: RoomPosition, to: RoomXY, grid: &HashMap<RoomXY, RoomPart>) -> Route {
    (to, find_path_in_room(from, to, 0, closest_in_room_range(grid)))
}

fn cmp_routes(r1: &Route, r2: &Route, from: Position) -> std::cmp::Ordering {
    use std::cmp::Ordering::Equal;
    match r1.1.len().cmp(&r2.1.len()) {
        Equal => {
            let x1_diff = r1.0.x.u8().abs_diff(from.x().u8());
            let y1_diff = r1.0.y.u8().abs_diff(from.y().u8());

            let x2_diff = r2.0.x.u8().abs_diff(from.x().u8());
            let y2_diff = r2.0.y.u8().abs_diff(from.y().u8());

            (x1_diff + y1_diff).cmp(&(x2_diff + y2_diff))
            // r1.0.get_range_to(from.xy()).cmp(&r2.0.get_range_to(from.xy()))
        }
        other => other,
    }
}

fn walkable_neighbors(
    xy: RoomXY,
    grid: &HashMap<RoomXY, RoomPart>,
) -> impl Iterator<Item = RoomXY> + use<'_> {
    xy.neighbors()
        .into_iter()
        .filter(|neighbor| grid.get(neighbor).is_some_and(|part| !part.is_wall()))
}

//todo guide cell could be an exit from farm to a base
fn guide_cell(
    ctrl: RoomXY,
    sources: &[RoomXY],
    rect: OuterRectangle,
) -> Result<RoomXY, RoomPlannerError> {
    once(ctrl)
        .chain(sources.iter().copied())
        .find(|xy| outside_rect(*xy, rect))
        .map_or_else(|| rect_center(rect).map_err(|_| RoomPlannerError::GiudePointNotFound), Ok)
}

fn rect_center(rect: OuterRectangle) -> Result<RoomXY, OutOfBoundsError> {
    let x = RoomCoordinate::new(rect.0 + rect.2 / 2)?;
    let y = RoomCoordinate::new(rect.1 + rect.3 / 2)?;
    Ok(RoomXY::new(x, y))
}

#[derive(Debug, Clone)]
pub struct Perimeter {
    rect: OuterRectangle,
    walls: Vec<RoomXY>,
    ramparts: Vec<RoomXY>,
}

impl Perimeter {
    pub fn new(rect: OuterRectangle, walls: &Walls) -> Self {
        let (x0, y0, x1, y1) = rect;
        let mut path: Vec<RoomXY> = Vec::new();

        // from top_left to top_right
        path.extend(walk_border(
            unsafe { RoomXY::unchecked_new(x0, y0) },
            unsafe { RoomXY::unchecked_new(x1, y0) },
            walls,
        ));

        // from top_right to bottom_right
        if let Some(start) = path.last().copied() {
            path.extend(walk_border(start, unsafe { RoomXY::unchecked_new(x1, y1) }, walls));
        }

        // from bottom_right to bottom_left
        if let Some(start) = path.last().copied() {
            path.extend(walk_border(start, unsafe { RoomXY::unchecked_new(x0, y1) }, walls));
        }

        // from bottom_left to top_left
        if let Some(start) = path.last().copied() {
            path.extend(walk_border(start, unsafe { RoomXY::unchecked_new(x0, y0) }, walls));
        }

        // Split into natural vs ramparts
        let mut natural_walls = Vec::new();
        let mut ramparts = Vec::new();
        for p in path {
            if is_wall(walls, p) {
                natural_walls.push(p);
            } else {
                ramparts.push(p);
            }
        }
        Self { walls: natural_walls, ramparts, rect: (x0, y0, x1, y1) }
    }

    const fn rectangle(&self) -> OuterRectangle {
        self.rect
    }

    fn ramparts(&self) -> &[RoomXY] {
        &self.ramparts
    }

    fn near_rampart(&self, xy: RoomXY) -> bool {
        self.ramparts.iter().any(|rampart| rampart.get_range_to(xy) == 1)
    }

    fn full_path(&self) -> impl Iterator<Item = &RoomXY> {
        self.walls.iter().chain(self.ramparts.iter())
    }

    fn full_path_sorted(&self) -> Vec<RoomXY> {
        self.full_path()
            .sorted_by(|xy1, xy2| match Ord::cmp(&xy1.y.u8(), &xy2.y.u8()) {
                Ordering::Equal => Ord::cmp(&xy1.x.u8(), &xy2.x.u8()),
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
            })
            .copied()
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RoadConfig {
    dash: Dash,
    row1: Variable,
    row2: Variable,
    turn: Turn,
}

#[derive(Debug)]
pub struct CentralSquare {
    cross_road: RoomXY,
    guide_dir: Direction,
    squares: HashMap<Direction, Square>,
}

#[derive(Debug)]
pub struct Square {
    center: RoomXY,
    sides: Vec<RoomXY>,
}

impl Eq for Square {}
impl PartialEq for Square {
    fn eq(&self, other: &Self) -> bool {
        self.center == other.center //&& self.sides.len() == other.sides.len()
    }
}

impl Square {
    pub const fn new(center: RoomXY, sides: Vec<RoomXY>) -> Self {
        Self { center, sides }
    }

    pub const fn center(&self) -> &RoomXY {
        &self.center
    }

    pub const fn rounded(&self) -> bool {
        self.sides.len() == 8
    }

    pub fn try_round(&self) -> Option<RoomXY> {
        if self.sides.len() == 7 {
            square_sides(&self.center, 1)
                .find(|xy| !self.sides.contains(xy))
                .filter(|vertex| !vertex.is_near_to(self.center)) //we can't round if distance == 1
                .and_then(|vertex| {
                    vertex
                        .get_direction_to(self.center)
                        .map(|direction| vertex.saturating_add_direction(direction))
                })
        } else {
            None
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = RoomXY> {
        [
            self.center,
            self.center.saturating_add((0, -1)),
            self.center.saturating_add((0, 1)),
            self.center.saturating_add((1, 0)),
            self.center.saturating_add((-1, 0)),
        ]
        .into_iter()
    }

    pub fn is_empty(&self, walls: &Walls, spawn: Option<&RoomXY>) -> bool {
        [
            self.center,
            self.center.saturating_add((0, -1)),
            self.center.saturating_add((0, 1)),
            self.center.saturating_add((1, 0)),
            self.center.saturating_add((-1, 0)),
        ]
        .iter()
        .all(|xy| !is_wall(walls, *xy) && spawn.is_none_or(|spawn_xy| xy != spawn_xy))
    }
}

#[cfg(test)]
mod tests {
    use screeps::RoomXY;

    use crate::rooms::state::constructions::{owned::smallest_perimeter, tests::WALLS};

    #[test]
    fn plan_room_test() {
        let sources = sources();
        let initial_spawn = None;

        let perimeter = smallest_perimeter(initial_spawn, &sources, &WALLS).unwrap();
        assert_eq!(perimeter.rectangle(), (27, 8, 45, 26));
    }

    pub fn sources() -> Vec<RoomXY> {
        unsafe { vec![RoomXY::unchecked_new(8, 23), RoomXY::unchecked_new(30, 15)] }
    }
}
