use std::collections::HashMap;

use screeps::pathfinder::SearchGoal;
use screeps::{Position, RoomXY};

use super::place_for_container;
use crate::movement::callback::construction_single_room;
use crate::movement::find_many_in_room;
use crate::rooms::state::constructions::{
    PlannedCell, RoomPart, RoomPlan, RoomPlannerError, RoomStructure,
};

pub fn plan(
    storage: Position,
    mineral: RoomXY,
    grid: &HashMap<RoomXY, RoomPart>,
    room_plan: &mut RoomPlan,
) -> Result<(), RoomPlannerError> {
    let (planned_roads, planned_structures) = room_plan.partition_by_roads_or_not();

    let container = place_for_container(
        storage,
        &mineral,
        planned_roads.iter().map(|c| c.xy).collect(),
        planned_structures.iter().map(|c| c.xy).collect(),
        grid,
    )
    .and_then(|xy| {
        grid.get(&xy).map(|part| PlannedCell::new(xy, RoomStructure::Container(*part), 6, None))
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
        return Err(RoomPlannerError::MineralPlacementFailure);
    }

    let road = search_result.path().into_iter().rev().enumerate().map(|(i, step)| {
        let distance = if grid.get(&step.xy()).is_some_and(|part| part.is_internal()) {
            0
        } else {
            let cell = PlannedCell::new(step.xy(), RoomStructure::Road(i), 6, None);
            planned_roads
                .get(&cell)
                .map_or(i, |cell| match cell.structure {
                    RoomStructure::Road(distance) => distance + i,
                    _ => i,
                })
        };
        PlannedCell::new(step.xy(), RoomStructure::Road(distance), 6, None)
    });
    room_plan.add_cells(road);
    room_plan.add_cell(PlannedCell::new(mineral, RoomStructure::Extractor, 6, None));
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use crate::rooms::constructions::tests::{mineral, room_name, sources,
// spawn};     use crate::rooms::constructions::blueprints::tests::{perimeter,
// grid};     use super::*;

//     #[test]
//     fn mineral_plan_test() {
//         let spawn = spawn();
//         let sources = sources();
//         let mineral = mineral();
//         let cross_road = unsafe { RoomXY::unchecked_new(22, 26) };

//         let perimeter = perimeter(spawn, &sources);
//         let grid = grid(&perimeter);

//         let cells = plan(&cross_road, mineral, &grid, iter::empty())
//             .expect("expect ctrl planned cells");

//         cells.iter()
//             .for_each(|cell| println!("cell: {:?}", cell));
//         // assert_eq!(7, cells.len(), "expect 7 cells");
//     }
// }
