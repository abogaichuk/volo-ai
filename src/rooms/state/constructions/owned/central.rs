use screeps::{RoomXY, Direction};
use std::collections::{HashMap, HashSet};
use crate::rooms::state::constructions::{
    LabStatus, LinkType, PlannedCell, RoomPlannerError, RoomStructure, Walls, xy_util::{clockwise_dir, counter_clockwise_dir, square_sides}
};
use super::{CentralSquare, Square};

pub fn central_square(
    target: &RoomXY,
    spawn: Option<RoomXY>,
    roads: &HashSet<RoomXY>,
    squares: &mut Vec<Square>,
    walls: &Walls) -> Result<CentralSquare, RoomPlannerError>
{
    let mut places = Vec::new();

    let centres: HashSet<RoomXY> = squares.iter()
        .filter(|square| square.rounded())
        .map(|square| *square.center())
        .collect();

    for road in roads {
        let neighbor_centres = [
            (Direction::Top, road.saturating_add((0, -2))),
            (Direction::Bottom, road.saturating_add((0, 2))),
            (Direction::Right, road.saturating_add((2, 0))),
            (Direction::Left, road.saturating_add((-2, 0)))
        ];


        if neighbor_centres.iter().all(|xy| centres.contains(&xy.1)) {
            let squares: HashMap<Direction, Square> = neighbor_centres.into_iter()
                .map(|(dir, center)| {
                    let sides = square_sides(&center, 1).collect();
                    (dir, Square::new(center, sides))
                })
                .filter(|(_, square)| square.is_empty(walls, spawn.as_ref()))
                .collect();

            if squares.len() == 4 && let Some(guide_direction) = road.get_direction_to(*target)
            {
                places.push(CentralSquare::new(*road, guide_direction, squares));
            }
        }
    }

    if let Some(central) = places.into_iter()
        .min_by_key(|place| place.cross_road().get_range_to(*target))
    {
        let placed: Vec<&Square> = central.squares().values().collect();
        squares.retain(|s| !placed.contains(&s));

        Ok(central)
    } else {
        Err(RoomPlannerError::CentralSquareNotFound)
    }
}

impl CentralSquare {
    pub fn new(cross_road: RoomXY, guide_dir: Direction, squares: HashMap<Direction, Square>) -> Self {
        Self { cross_road, guide_dir, squares }
    }

    pub fn guide_dir(&self) -> Direction {
        self.guide_dir
    }

    pub fn cross_road(&self) -> &RoomXY {
        &self.cross_road
    }

    pub fn squares(&self) -> &HashMap<Direction, Square> {
        &self.squares
    }

    pub fn plan(&self) -> Result<HashSet<PlannedCell>, RoomPlannerError> {
        let mut planed_cells = HashSet::new();
        let (storage_sqr_dir, terminal_sqr_dir) = match self.guide_dir {
            Direction::BottomLeft | Direction::BottomRight | Direction::TopLeft | Direction::TopRight =>
                (clockwise_dir(self.guide_dir ), counter_clockwise_dir(self.guide_dir )),
            _ => (self.guide_dir , clockwise_dir(clockwise_dir(self.guide_dir )))
        };
        let (nuker_sqr_dir, ps_sqr_dir) = (-storage_sqr_dir, -terminal_sqr_dir);

        let storage_square = self.squares.get(&storage_sqr_dir)
            .ok_or(RoomPlannerError::CentralSquarePlacementError)?;
        let storage = PlannedCell::new(
            storage_square.center.saturating_add_direction(terminal_sqr_dir), //storage close to terminal
            RoomStructure::Storage,
            4,
            None);
        let lab_ouput1 = PlannedCell::new(
            storage_square.center.saturating_add_direction(-storage_sqr_dir),
            RoomStructure::Lab(LabStatus::Output),
            6,
            None);
        let link1 = PlannedCell::new(
            storage_square.center.saturating_add_direction(-terminal_sqr_dir),
            RoomStructure::Link(LinkType::Sender),
            5,
            None);
        let link2 = PlannedCell::new(
            storage_square.center.saturating_add_direction(storage_sqr_dir),
            RoomStructure::Link(LinkType::Receiver),
            6,
            None);
        let road = PlannedCell::new(
            storage_square.center,
            RoomStructure::Road(0),
            1,
            None);
        
        planed_cells.insert(storage);
        planed_cells.insert(lab_ouput1);
        planed_cells.insert(link1);
        planed_cells.insert(link2);
        planed_cells.insert(road);

        let terminal_square = self.squares.get(&terminal_sqr_dir)
            .ok_or(RoomPlannerError::CentralSquarePlacementError)?;
        let terminal = PlannedCell::new(
            terminal_square.center.saturating_add_direction(storage_sqr_dir), //terminal close to storage
            RoomStructure::Terminal,
            6,
            None);
        let lab_ouput2 = PlannedCell::new(
            terminal_square.center.saturating_add_direction(-terminal_sqr_dir),
            RoomStructure::Lab(LabStatus::Output),
            7,
            None);
        let factory = PlannedCell::new(
            terminal_square.center.saturating_add_direction(terminal_sqr_dir),
            RoomStructure::Factory,
            7,
            None);
            //todo change to my structure Empty -> reserved by pc
        let extension = PlannedCell::new(
            terminal_square.center.saturating_add_direction(-storage_sqr_dir),
            RoomStructure::Empty,
            8,
            None);
        let road = PlannedCell::new(
            terminal_square.center,
            RoomStructure::Road(0),
            1,
            None);

        planed_cells.insert(terminal);
        planed_cells.insert(lab_ouput2);
        planed_cells.insert(factory);
        planed_cells.insert(extension);
        planed_cells.insert(road);

        let nuker_square = self.squares.get(&nuker_sqr_dir)
            .ok_or(RoomPlannerError::CentralSquarePlacementError)?;
        let nuker = PlannedCell::new(
            nuker_square.center.saturating_add_direction(nuker_sqr_dir), //opposite direction from storage
            RoomStructure::Nuker,
            8,
            None);
        let lab_input1 = PlannedCell::new(
            nuker_square.center.saturating_add_direction(-nuker_sqr_dir),
            RoomStructure::Lab(LabStatus::Input),
            6,
            None);
        let lab_ouput3 = PlannedCell::new(
            nuker_square.center,
            RoomStructure::Lab(LabStatus::Output),
            7,
            None);
        let lab_ouput4 = PlannedCell::new(
            nuker_square.center.saturating_add_direction(-ps_sqr_dir),
            RoomStructure::Lab(LabStatus::Output),
            7,
            None);
        let lab_ouput5 = PlannedCell::new(
            nuker_square.center.saturating_add_direction(ps_sqr_dir),
            RoomStructure::Lab(LabStatus::Output),
            8,
            None);

        planed_cells.insert(nuker);
        planed_cells.insert(lab_input1);
        planed_cells.insert(lab_ouput3);
        planed_cells.insert(lab_ouput4);
        planed_cells.insert(lab_ouput5);

        let ps_square = self.squares.get(&ps_sqr_dir)
            .ok_or(RoomPlannerError::CentralSquarePlacementError)?;
        let ps = PlannedCell::new(
            ps_square.center.saturating_add_direction(ps_sqr_dir), //opposite direction from terminal
            RoomStructure::PowerSpawn,
            8,
            None);
        let lab_input2 = PlannedCell::new(
            ps_square.center.saturating_add_direction(-ps_sqr_dir),
            RoomStructure::Lab(LabStatus::Input),
            6,
            None);
        let lab_ouput6 = PlannedCell::new(
            ps_square.center,
            RoomStructure::Lab(LabStatus::Output),
            8,
            None);
        let lab_ouput7 = PlannedCell::new(
            ps_square.center.saturating_add_direction(-nuker_sqr_dir),
            RoomStructure::Lab(LabStatus::Output),
            8,
            None);
        let lab_ouput8 = PlannedCell::new(
            ps_square.center.saturating_add_direction(nuker_sqr_dir),
            RoomStructure::Lab(LabStatus::Output),
            8,
            None);

        planed_cells.insert(ps);
        planed_cells.insert(lab_input2);
        planed_cells.insert(lab_ouput6);
        planed_cells.insert(lab_ouput7);
        planed_cells.insert(lab_ouput8);

        Ok(planed_cells)
    }
}