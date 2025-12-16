use log::warn;
use screeps::Direction;
use screeps::local::RoomXY;

use super::Perimeter;
use crate::rooms::state::constructions::xy_util::clockwise_dir;
use crate::rooms::state::constructions::{
    OuterRectangle, ROOM_SIZE, RoomPlannerError, Sat, Walls, is_wall,
};

const SAFE_RANGE: u8 = 3; // Chebyshev radius for "safe from perimeter"
const MIN_SAFE_CELLS: usize = 150;

const BUILD_MIN: u8 = 8;
const BUILD_MAX: u8 = 42;
const MIN_CORE: u8 = 13;
const MAX_CORE: u8 = 20;

pub(super) fn smallest_perimeter(
    spawn: Option<RoomXY>,
    sources: &[RoomXY],
    walls: &Walls,
) -> Result<Perimeter, RoomPlannerError> {
    let sat = build_sat(walls);
    minimal_rectangles(spawn, sources, &sat)
        .into_iter()
        .map(|rect| Perimeter::new(rect, walls))
        .next()
        .ok_or(RoomPlannerError::PerimeterCreationFailed)
}

pub(super) fn walk_border(from: RoomXY, to: RoomXY, walls: &Walls) -> Vec<RoomXY> {
    let mut path = Vec::new();
    if let Some(direction) = from.get_direction_to(to).filter(|dir| {
        matches!(dir, Direction::Right | Direction::Bottom | Direction::Left | Direction::Top)
    }) {
        let mut distance = from.get_range_to(to);
        let secondary_dir = clockwise_dir(clockwise_dir(direction));
        let mut cur = from;

        while distance > 0 {
            let next = cur.saturating_add_direction(direction);
            if is_wall(walls, cur) {
                path.push(cur); //todo push next if it's last?
                if !is_wall(walls, next) {
                    let another_next = cur.saturating_add_direction(secondary_dir);
                    let p = cut_edge(another_next, direction, secondary_dir, distance, walls);
                    if !p.is_empty() {
                        //better path found
                        path.extend(p);
                        break;
                    }
                }
            } else {
                //rampart here, check secondary direction is natural wall
                path.push(cur);
                let another_next = cur.saturating_add_direction(secondary_dir);
                if !is_wall(walls, next) && is_wall(walls, another_next) {
                    //try cut to the bottom
                    let p = cut_edge(another_next, direction, secondary_dir, distance, walls);
                    if !p.is_empty() {
                        //better path found
                        path.extend(p);
                        break;
                    }
                }
            }

            if distance == 1 {
                path.push(next);
            }

            distance -= 1;
            cur = next;
        }
    } else {
        warn!("ivalid direction from: {}, to: {}", from, to);
    }
    path
}

fn cut_edge(
    start: RoomXY,
    primary_dir: Direction,
    secondary_dir: Direction,
    distance: u8,
    walls: &Walls,
) -> Vec<RoomXY> {
    let mut path = Vec::new();

    let next = start.saturating_add_direction(primary_dir);
    if distance == 0 {
        path.push(start);
    } else if is_wall(walls, next) {
        let segment = cut_edge(next, primary_dir, secondary_dir, distance - 1, walls);
        if !segment.is_empty() {
            //reached the distance
            path.push(start);
            path.extend(segment);
        }
    } else {
        let another_next = start.saturating_add_direction(secondary_dir);
        let segments = if is_wall(walls, another_next) {
            cut_edge(another_next, primary_dir, secondary_dir, distance, walls)
        } else {
            Vec::new()
        };

        if !segments.is_empty() {
            //reached the distance
            path.push(start);
            path.extend(segments);
        } else if distance == 1 {
            // natural walls path not found, but next cell - last cell, set rampart
            path.push(start);
            path.push(next);
        }
    }
    path
}

/// Returns *all* rectangles with the smallest safe-core size that yield ≥
/// `TARGET_MIN_SAFE_CELLS`.
#[allow(clippy::similar_names)]
fn minimal_rectangles(spawn: Option<RoomXY>, sources: &[RoomXY], sat: &Sat) -> Vec<OuterRectangle> {
    let mut sizes: Vec<(u8, u8)> = Vec::new();
    for w in MIN_CORE..=MAX_CORE {
        for h in MIN_CORE..=MAX_CORE {
            sizes.push((w, h));
            if w != h {
                sizes.push((h, w));
            }
        }
    }
    sizes.sort_by_key(|&(w, h)| (u16::from(w) * u16::from(h), w.min(h)));

    let mut results_at_min_size: Option<Vec<OuterRectangle>> = None;

    for (w, h) in sizes {
        let outer_w: u8 = w + 2 * SAFE_RANGE;
        let outer_h: u8 = h + 2 * SAFE_RANGE;

        let max_x0 = BUILD_MAX.saturating_sub((outer_w) - 1);
        let max_y0 = BUILD_MAX.saturating_sub((outer_h) - 1);

        let mut found: Vec<OuterRectangle> = Vec::new();
        for y0 in BUILD_MIN..=max_y0 {
            for x0 in BUILD_MIN..=max_x0 {
                let x1 = x0 + outer_w - 1;
                let y1 = y0 + outer_h - 1;

                // at least 1 source should be inside or near the rectangle
                if !sources.iter().any(|&s| source_near_rect(s, x0, y0, x1, y1, 1)) {
                    continue;
                }

                // core box: shrink by SAFE_RANGE on each side
                let cx0 = x0.saturating_add(SAFE_RANGE);
                let cy0 = y0.saturating_add(SAFE_RANGE);
                let cx1 = x1.saturating_sub(SAFE_RANGE);
                let cy1 = y1.saturating_sub(SAFE_RANGE);

                if spawn.is_some_and(|xy| {
                    xy.x.u8() < cx0 || xy.x.u8() > cx1 || xy.y.u8() < cy0 || xy.y.u8() > cy1
                }) {
                    continue;
                }

                // core dimensions
                let core_w = (cx1 - cx0 + 1) as usize;
                let core_h = (cy1 - cy0 + 1) as usize;

                let core_area = core_w * core_h;

                // safe_cells = core_area - (# natural wall cells in the core)
                let walls_in_core = rect_walls(sat, cx0, cy0, cx1, cy1) as usize;
                let safe_cells = core_area.saturating_sub(walls_in_core);

                if safe_cells < MIN_SAFE_CELLS {
                    continue;
                }

                found.push((x0, y0, x1, y1));
            }
        }

        if !found.is_empty() {
            results_at_min_size = Some(found);
            break; // return *minimal* core shapes only
        }
    }

    results_at_min_size.unwrap_or_default()
}

/// Rectangle is valid if source inside  or Chebyshev-distance ≤ r to the
/// rectangle.
fn source_near_rect(source: RoomXY, x0: u8, y0: u8, x1: u8, y1: u8, r: u8) -> bool {
    let sx = i32::from(source.x.u8());
    let sy = i32::from(source.y.u8());
    let (x0, y0, x1, y1) = (i32::from(x0), i32::from(y0), i32::from(x1), i32::from(y1));
    let r = i32::from(r);
    let dx = if sx < x0 {
        x0 - sx
    } else if sx > x1 {
        sx - x1
    } else {
        0
    };
    let dy = if sy < y0 {
        y0 - sy
    } else if sy > y1 {
        sy - y1
    } else {
        0
    };
    dx.max(dy) <= r
}

/// apply inclusion–exclusion rectangle sum:
/// returns number of natural-wall cells in [x0..=x1] × [y0..=y1].
#[inline]
const fn rect_walls(sat: &Sat, x0: u8, y0: u8, x1: u8, y1: u8) -> u16 {
    let (x0, y0, x1, y1) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);

    let a = sat[y1][x1];
    let b = if y0 > 0 { sat[y0 - 1][x1] } else { 0 };
    let c = if x0 > 0 { sat[y1][x0 - 1] } else { 0 };
    let d = if y0 > 0 && x0 > 0 { sat[y0 - 1][x0 - 1] } else { 0 };

    a + d - b - c
}

#[inline]
pub fn build_sat(walls: &[[bool; ROOM_SIZE as usize]; ROOM_SIZE as usize]) -> Sat {
    let mut sat = [[0u16; ROOM_SIZE as usize]; ROOM_SIZE as usize];

    for y in 0..ROOM_SIZE as usize {
        for x in 0..ROOM_SIZE as usize {
            let v = u16::from(walls[y][x]); // 0 or 1

            // sums from neighbors; use 0 when on the border
            let up = if y > 0 { sat[y - 1][x] } else { 0 };
            let left = if x > 0 { sat[y][x - 1] } else { 0 };
            let diag = if y > 0 && x > 0 { sat[y - 1][x - 1] } else { 0 };

            sat[y][x] = v + up + left - diag;
        }
    }

    sat
}

// #[cfg(test)]
// mod tests {
//     use crate::rooms::constructions::tests::{WALLS, sources, spawn};
//     use super::*;

//     #[test]
//     fn smallest_perimeter_test() {
//         let spawn = spawn();
//         let sources = sources();
//         let perimeter = smallest_perimeter(spawn, &sources,
// &WALLS).expect("expect perimeter");

//         assert_eq!((13, 17, 31, 36), perimeter.rectangle(), "invalid
// rectangle");         assert_eq!(26, perimeter.ramparts().len(), "invalid
// ramparts len");     }
// }
