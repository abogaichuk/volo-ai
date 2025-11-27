use screeps::RoomXY;
use std::{cmp::Reverse, collections::HashMap};
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
        .min_by_key(|(xy, _)| Reverse(xy.get_range_to(cross_road)))
        .map(|xy| PlannedCell::new(*xy.0, RoomStructure::Observer, 8, None))
        .into_iter()
    );
}