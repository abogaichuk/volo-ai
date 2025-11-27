use screeps::{Direction, RoomXY};
use itertools::Itertools;
use crate::rooms::state::constructions::{
    xy_util::counter_clockwise_dir, PlannedCell, RoomPlan,
    RoomPlannerError, RoomStructure, is_wall, Walls, owned::{CentralSquare, Square}
};

pub fn plan(spawns: &[RoomXY], room_plan: &mut RoomPlan) {
        room_plan.add_cells(
            spawns.iter().enumerate()
            .map(|(i, xy)| {
                let ctrl_lvl = if i == 0 {
                    1
                } else if i == 1 {
                    7
                } else {
                    8
                };
                PlannedCell::new(*xy, RoomStructure::Spawn, ctrl_lvl, None)
            })
        );
    }

pub fn spawn_space(
    central: &CentralSquare,
    spawn: Option<RoomXY>,
    squares: &[Square],
    walls: &Walls) -> Result<Vec<RoomXY>, RoomPlannerError>
{
    let mut spawns = Vec::with_capacity(3);
    spawns.extend(spawn);

    let direction = match central.guide_dir() {
        Direction::BottomLeft | Direction::BottomRight | Direction::TopLeft | Direction::TopRight => -central.guide_dir(),
        _ => counter_clockwise_dir(-central.guide_dir())
    };
    // I want to place a spawns closest to labs so I shifted crossroad center
    let shifted_center = central.cross_road().saturating_add_direction(direction);

    let candidates: Vec<RoomXY> = squares.iter()
        .filter(|square| !square.cells().any(|cell| spawns.contains(&cell)))
        .sorted_by_key(|square| square.center().get_range_to(shifted_center))
        .take(4)
        .flat_map(|square| square.cells())
        .filter(|cell| !is_wall(walls, cell))
        .collect();

    let mut used = vec![false; candidates.len()];
    if choose_spawn_places(spawns.len(), 3, &mut spawns, &candidates, &mut used) {
        Ok(spawns)
    } else {
        Err(RoomPlannerError::SpawnPlaceNotFound)
    }
}

fn choose_spawn_places(
    i: usize,
    count: usize,
    chosen: &mut Vec<RoomXY>,
    candidates: &[RoomXY],
    used: &mut Vec<bool>,
) -> bool {
    if i == count { return true; }

    for (idx, &p) in candidates.iter().enumerate() {
        if used[idx] { continue; }
        // must satisfy distance to ALL previously chosen
        if chosen.iter().all(|&q| p.get_range_to(q) > 2) {
            used[idx] = true;
            chosen.push(p);
            if choose_spawn_places(i + 1, count, chosen, candidates, used) {
                return true;
            }
            chosen.pop();
            used[idx] = false;
        }
    }
    false
}