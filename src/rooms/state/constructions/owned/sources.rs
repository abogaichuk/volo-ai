use std::collections::HashMap;

use itertools::Itertools;
use log::info;
use screeps::pathfinder::SearchGoal;
use screeps::{HasPosition, Position, RoomXY};

use crate::movement::callback::construction_single_room;
use crate::movement::find_many_in_room;
use crate::rooms::state::constructions::owned::{
    place_for_container, walkable_neighbors, walkable_range,
};
use crate::rooms::state::constructions::{
    LinkType, PlannedCell, RoomPart, RoomPlan, RoomPlannerError, RoomStructure,
};

pub fn plan(
    storage: Position,
    sources: &[RoomXY],
    grid: &HashMap<RoomXY, RoomPart>,
    room_plan: &mut RoomPlan,
) -> Result<(), RoomPlannerError> {
    for (i, source) in sources
        .iter()
        .sorted_by_key(|s| walkable_range(storage.into(), **s, grid).1.len())
        .rev()
        .enumerate()
    {
        let (planned_roads, planned_structures) = room_plan.partition_by_roads_or_not();
        let container = place_for_container(
            storage,
            source,
            planned_roads.iter().map(|c| c.xy).collect(),
            planned_structures.iter().map(|c| c.xy).collect(),
            grid,
        )
        .and_then(|xy| {
            grid.get(&xy).map(|part| {
                PlannedCell::new(
                    xy,
                    RoomStructure::Container(*part),
                    3,
                    if i == 0 { Some(8) } else { Some(7) },
                )
            })
        })
        .ok_or(RoomPlannerError::SourcePlacementFailure)?;
        info!("{} container at: {}", storage.pos().room_name(), container.xy);

        let goals = planned_roads.iter().map(|cell| {
            SearchGoal::new(Position::new(cell.xy.x, cell.xy.y, storage.room_name()), 0)
        });
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
                    .map(|cell| match cell.structure {
                        RoomStructure::Road(distance) => distance + i,
                        _ => i,
                    })
                    .unwrap_or(i)
            };
            PlannedCell::new(step.xy(), RoomStructure::Road(distance), 0, None)
        });
        room_plan.add_cells(road);

        let link = walkable_neighbors(&container.xy, grid)
            .filter(|xy| !room_plan.is_occupied(*xy))
            .min_by_key(|xy| xy.get_range_to(storage.xy()))
            .map(|xy| {
                PlannedCell::new(
                    xy,
                    RoomStructure::Link(LinkType::Source),
                    if i == 0 { 6 } else { 8 },
                    None,
                )
            })
            .ok_or(RoomPlannerError::SourcePlacementFailure)?;

        room_plan.add_cell(container);
        room_plan.add_cell(link);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // use crate::rooms::constructions::tests::{sources, spawn};
    // use crate::rooms::constructions::blueprints::tests::{perimeter, grid};
    // use super::*;

    // #[test]
    // fn sources_plan_test() {
    //     let spawn = spawn();
    //     let sources = sources();
    //     let cross_road = unsafe { RoomXY::unchecked_new(22, 26) };

    //     let perimeter = perimeter(spawn, &sources);
    //     let grid = grid(&perimeter);

    //     let cells = plan(&cross_road, &sources, &grid).expect("expect ctrl
    // planned cells");

    //     assert_eq!(7, cells.len(), "expect 7 cells");
    // }
}
