use std::collections::HashMap;

use screeps::pathfinder::SearchGoal;
use screeps::{Position, RoomXY};

use super::{place_for_container, walkable_neighbors};
use crate::movement::callback::construction_single_room;
use crate::movement::find_many_in_room;
use crate::rooms::state::constructions::{
    LinkType, PlannedCell, RoomPart, RoomPlan, RoomPlannerError, RoomStructure,
};

pub fn plan(
    storage: Position,
    ctrl: RoomXY,
    grid: &HashMap<RoomXY, RoomPart>,
    room_plan: &mut RoomPlan,
) -> Result<(), RoomPlannerError> {
    let (planned_roads, planned_structures) = room_plan.partition_by_roads_or_not();

    let container = place_for_container(
        storage,
        &ctrl,
        planned_roads.iter().map(|c| c.xy).collect(),
        planned_structures.iter().map(|c| c.xy).collect(),
        grid,
    )
    .and_then(|xy| {
        grid.get(&xy).map(|part| PlannedCell::new(xy, RoomStructure::Container(*part), 3, Some(5)))
    })
    .ok_or(RoomPlannerError::ControllerPlacementFailure)?;

    room_plan.add_cell(container);

    let goals = planned_roads
        .iter()
        .map(|cell| SearchGoal::new(Position::new(cell.xy.x, cell.xy.y, storage.room_name()), 0));
    let search_result = find_many_in_room(
        Position::new(container.xy.x, container.xy.y, storage.room_name()),
        goals,
        construction_single_room(planned_structures.iter().map(|c| c.xy).collect(), grid),
    );

    if search_result.incomplete() {
        return Err(RoomPlannerError::SourcePlacementFailure);
    }

    let road = search_result.path().into_iter().rev().enumerate().map(|(i, step)| {
        let distance = if grid.get(&step.xy()).is_some_and(|part| part.is_internal()) {
            0
        } else {
            let cell = PlannedCell::new(step.xy(), RoomStructure::Road(i), 0, None);
            planned_roads
                .get(&cell)
                .map_or(i, |cell| match cell.structure {
                    RoomStructure::Road(distance) => distance + i,
                    _ => i,
                })
        };
        PlannedCell::new(step.xy(), RoomStructure::Road(distance), 0, None)
    });
    room_plan.add_cells(road);

    let link = walkable_neighbors(&container.xy, grid)
        .find(|xy| xy.is_near_to(ctrl))
        .map(|xy| PlannedCell::new(xy, RoomStructure::Link(LinkType::Ctrl), 5, None))
        .ok_or(RoomPlannerError::ControllerPlacementFailure)?;

    room_plan.add_cell(link);
    room_plan.add_cells(
        walkable_neighbors(&ctrl, grid)
            .map(|xy| PlannedCell::new(xy, RoomStructure::Rampart(false), 8, None)),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    // use crate::rooms::constructions::tests::{sources, spawn, ctrl};
    // use crate::rooms::constructions::blueprints::tests::{perimeter, grid};
    // use super::*;

    // #[test]
    // fn ctrl_plan_test() {
    //     let spawn = spawn();
    //     let ctrl = ctrl();
    //     let sources = sources();
    //     let cross_road = unsafe { RoomXY::unchecked_new(22, 26) };

    //     let perimeter = perimeter(spawn, &sources);
    //     let grid = grid(&perimeter);

    //     let cells = plan(&cross_road, &ctrl, &grid).expect("expect ctrl
    // planned cells");

    //     assert_eq!(
    //         cells.iter().filter(|c| matches!(c.structure,
    // RoomStructure::Rampart(_))).count(),         4,
    //         "expect 4 ramparts"
    //     );
    //     assert_eq!(
    //         cells.iter().filter(|c| matches!(c.structure,
    // RoomStructure::Link(_))).count(),         1,
    //         "expect 1 link"
    //     );
    //     assert_eq!(
    //         cells.iter().filter(|c| matches!(c.structure,
    // RoomStructure::Container(_))).count(),         1,
    //         "expect 1 container"
    //     );
    // }
}
