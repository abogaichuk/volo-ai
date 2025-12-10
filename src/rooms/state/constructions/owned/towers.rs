use std::cmp::Reverse;
use std::collections::HashMap;
use std::iter;

use itertools::Itertools;
use screeps::{RoomCoordinate, RoomXY};

use crate::rooms::state::constructions::owned::Perimeter;
use crate::rooms::state::constructions::{PlannedCell, RoomPart, RoomPlan, RoomStructure};

pub fn plan(perimeter: &Perimeter, grid: &HashMap<RoomXY, RoomPart>, room_plan: &mut RoomPlan) {
    let reds = red_parts_near_ramparts(perimeter, grid);
    let candidates: Vec<(RoomXY, usize)> = grid
        .iter()
        .filter_map(|(xy, part)| if part.is_yellow() { Some(*xy) } else { None })
        .map(|yellow| {
            let under_attack_count =
                reds.iter().filter(|red| red.get_range_to(yellow) == 3).count();
            (yellow, under_attack_count)
        })
        .sorted_by_key(|(_, len)| Reverse(*len))
        .collect();

    room_plan.add_cells(select_best_spread(candidates, 6, 4).enumerate().flat_map(|(i, xy)| {
        let ctrl_lvl = if i == 0 {
            3
        } else if i == 1 {
            5
        } else if i == 2 {
            7
        } else {
            8
        };
        [
            PlannedCell::new(xy, RoomStructure::Tower, ctrl_lvl, None),
            PlannedCell::new(xy, RoomStructure::Rampart(false), 8, None),
        ]
        .into_iter()
    }));
}

/// Exact selector: maximize total score with pairwise Chebyshev distance >
/// `min_exclusive`. Returns up to `k` items (fewer if constraints force it).
fn select_best_spread(
    candidates: Vec<(RoomXY, usize)>,
    k: usize,
    min_exclusive: u8,
) -> Box<dyn Iterator<Item = RoomXY>> {
    if candidates.is_empty() || k == 0 {
        return Box::new(iter::empty());
    }

    let n = candidates.len();

    // Precompute conflict matrix (true = too close).
    let mut conflict = vec![vec![false; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let bad = candidates[i].0.get_range_to(candidates[j].0) <= min_exclusive;
            conflict[i][j] = bad;
            conflict[j][i] = bad;
        }
    }

    // Best-so-far state.
    let mut best_sum: usize = 0;
    let mut best_idxs: Vec<usize> = Vec::with_capacity(k);

    // Iterative DFS stack: (next_index, current_sum, chosen_indices)
    let mut stack: Vec<(usize, usize, Vec<usize>)> = vec![(0, 0, Vec::new())];

    while let Some((i, cur_sum, chosen)) = stack.pop() {
        // Update best (prefer larger set when sums tie).
        if cur_sum > best_sum || (cur_sum == best_sum && chosen.len() > best_idxs.len()) {
            best_sum = cur_sum;
            // best_idxs = chosen.clone();
            best_idxs.clone_from(&chosen);
        }

        // Stop if full or out of candidates.
        if chosen.len() == k || i == n {
            continue;
        }

        let remain_slots = k - chosen.len();

        // Fast optimistic bound (ignores conflicts).
        let optimistic = cur_sum
            + candidates.iter().take(n).skip(i).take(remain_slots).map(|&(_, s)| s).sum::<usize>();

        if optimistic <= best_sum {
            continue; // prune
        }

        // Branch: take i (if compatible with all chosen).
        if chosen.iter().all(|&p| !conflict[p][i]) {
            let mut next = chosen.clone();
            next.push(i);
            stack.push((i + 1, cur_sum + candidates[i].1, next));
        }

        // Branch: skip i.
        stack.push((i + 1, cur_sum, chosen));
    }

    Box::new(best_idxs.into_iter().map(move |idx| candidates[idx].0))
}

fn red_parts_near_ramparts(perimeter: &Perimeter, grid: &HashMap<RoomXY, RoomPart>) -> Vec<RoomXY> {
    let mut attack_zones = Vec::new();

    let (x0, y0, x1, y1) = perimeter.rectangle();
    for y in y0 - 1..=y1 + 1 {
        for x in x0 - 1..=x1 + 1 {
            let xy = unsafe {
                RoomXY::new(RoomCoordinate::unchecked_new(x), RoomCoordinate::unchecked_new(y))
            };
            if grid.get(&xy).is_some_and(|part| part.is_red()) && perimeter.near_rampart(xy) {
                attack_zones.push(xy);
            }
        }
    }

    attack_zones
}
