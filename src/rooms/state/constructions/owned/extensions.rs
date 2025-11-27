use screeps::RoomXY;
use itertools::Itertools;
use std::collections::HashMap;
use crate::rooms::state::constructions::{PlannedCell, RoomPart, RoomPlan, RoomStructure};

pub fn plan(
    cross_road: RoomXY,
    grid: &HashMap<RoomXY, RoomPart>,
    plan: &mut RoomPlan)
{
    let occupied = plan.occupied();

    plan.add_cells(
        grid.iter()
            .filter(|(xy, part)| **part == RoomPart::Green && !occupied.contains(xy))
            .map(|(xy, _)| xy)
            .sorted_by_key(|xy| xy.get_range_to(cross_road))
            .enumerate()
            .filter_map(|(i, xy)| {
                if i < 5 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 2, None))
                } else if i < 10 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 3, None))
                } else if i < 20 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 4, None))
                } else if i < 30 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 5, None))
                } else if i < 40 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 6, None))
                } else if i < 50 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 7, None))
                } else if i < 60 {
                    Some(PlannedCell::new(*xy, RoomStructure::Extension, 8, None))
                } else {
                    None
                }
            })
    );
}