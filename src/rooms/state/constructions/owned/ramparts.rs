use crate::rooms::state::constructions::{PlannedCell, RoomPlan, RoomStructure};
use super::Perimeter;

pub fn plan(
    perimeter: &Perimeter,
    room_plan: &mut RoomPlan)
{
    room_plan.add_cells(
        perimeter.ramparts().iter()
            .map(|xy| PlannedCell::new(*xy, RoomStructure::Rampart(true), 4, None))
    );
}