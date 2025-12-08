use std::cmp::min;

use screeps::{Direction, RoomXY};

use super::OuterRectangle;

pub const ROOM_SIZE: usize = screeps::constants::extra::ROOM_SIZE as usize;
pub const ROOM_AREA: usize = ROOM_SIZE * ROOM_SIZE;

#[inline]
pub fn diagonal_neighbors(xy: &RoomXY) -> impl Iterator<Item = RoomXY> {
    [
        xy.saturating_add((1, 1)),
        xy.saturating_add((1, -1)),
        xy.saturating_add((-1, 1)),
        xy.saturating_add((-1, -1)),
    ]
    .into_iter()
}

#[inline]
pub fn outside_rect(xy: &RoomXY, rectangle: OuterRectangle) -> bool {
    let (x0, y0, x1, y1) = rectangle;
    xy.x.u8() < x0 || xy.x.u8() > x1 || xy.y.u8() < y0 || xy.y.u8() > y1
}

#[inline]
pub fn to_index(xy: RoomXY) -> usize {
    (xy.x.u8() as usize) + ROOM_SIZE * (xy.y.u8() as usize)
}

fn iter_xy() -> impl Iterator<Item = RoomXY> {
    (0..ROOM_AREA)
        .map(|i| unsafe { RoomXY::unchecked_new((i % ROOM_SIZE) as u8, (i / ROOM_SIZE) as u8) })
}

#[inline]
pub fn exit_distance(xy: RoomXY) -> u8 {
    min(
        min(xy.x.u8(), xy.y.u8()),
        min(ROOM_SIZE as u8 - 1 - xy.x.u8(), ROOM_SIZE as u8 - 1 - xy.y.u8()),
    )
}

#[inline]
pub fn square_sides(xy: &RoomXY, multiplier: i8) -> impl Iterator<Item = RoomXY> {
    [
        xy.saturating_add((0, 2 * multiplier)),
        xy.saturating_add((0, -2 * multiplier)),
        xy.saturating_add((2 * multiplier, 0)),
        xy.saturating_add((-2 * multiplier, 0)),
        xy.saturating_add((multiplier, multiplier)),
        xy.saturating_add((multiplier, -multiplier)),
        xy.saturating_add((-multiplier, multiplier)),
        xy.saturating_add((-multiplier, -multiplier)),
    ]
    .into_iter()
}

#[inline]
pub fn clockwise_dir(direction: Direction) -> Direction {
    match direction {
        Direction::Right => Direction::BottomRight,
        Direction::BottomRight => Direction::Bottom,
        Direction::Bottom => Direction::BottomLeft,
        Direction::BottomLeft => Direction::Left,
        Direction::Left => Direction::TopLeft,
        Direction::TopLeft => Direction::Top,
        Direction::Top => Direction::TopRight,
        Direction::TopRight => Direction::Right,
    }
}

#[inline]
pub fn counter_clockwise_dir(direction: Direction) -> Direction {
    match direction {
        Direction::Right => Direction::TopRight,
        Direction::TopRight => Direction::Top,
        Direction::Top => Direction::TopLeft,
        Direction::TopLeft => Direction::Left,
        Direction::Left => Direction::BottomLeft,
        Direction::BottomLeft => Direction::Bottom,
        Direction::Bottom => Direction::BottomRight,
        Direction::BottomRight => Direction::Right,
    }
}

#[cfg(test)]
mod tests {
    // use crate::rooms::constructions::tests::{sources, spawn};
    // use crate::rooms::constructions::blueprints::tests::{perimeter, grid};
    // use super::*;

    // #[test]
    // fn get_sides_test() {
    //     let xy = unsafe { RoomXY::unchecked_new(25, 25) };
    //     get_sides(&xy, 1)
    //         .for_each(|side| println!("side: {}", side));
    // }
}
