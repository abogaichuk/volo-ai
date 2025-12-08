use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use screeps::{RoomCoordinate, RoomXY};
use serde::{Deserialize, Serialize};

use super::{RoadConfig, Square};
use crate::rooms::state::constructions::xy_util::{diagonal_neighbors, square_sides};
use crate::rooms::state::constructions::{
    OuterRectangle, PlannedCell, RoomPart, RoomPlan, RoomPlannerError, RoomStructure,
};

pub fn best_net(
    rect: OuterRectangle,
    spawn: Option<RoomXY>,
    grid: &HashMap<RoomXY, RoomPart>,
) -> Result<RoadNet, RoomPlannerError> {
    all_road_configs()
        .into_iter()
        .map(|config| config.produce_net(rect, spawn, grid))
        .min_by_key(|roads_net| Reverse(roads_net.rank()))
        .ok_or(RoomPlannerError::RoadPlanFailure)
}

fn all_road_configs() -> Vec<RoadConfig> {
    vec![
        RoadConfig::new(Dash::new(Turn::First, Turn::First), false),
        RoadConfig::new(Dash::new(Turn::First, Turn::First), true),
        RoadConfig::new(Dash::new(Turn::First, Turn::Second), false),
        RoadConfig::new(Dash::new(Turn::First, Turn::Second), true),
        RoadConfig::new(Dash::new(Turn::Second, Turn::First), false),
        RoadConfig::new(Dash::new(Turn::Second, Turn::First), true),
        RoadConfig::new(Dash::new(Turn::Second, Turn::Second), false),
        RoadConfig::new(Dash::new(Turn::Second, Turn::Second), true),
    ]
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Offset {
    O0,
    O1,
    O2,
    O3,
}
impl Offset {
    fn predicate(&self) -> fn(u8) -> bool {
        match self {
            Offset::O0 => |x| x % 4 == 0,
            Offset::O1 => |x| x % 4 == 1,
            Offset::O2 => |x| x % 4 == 2,
            Offset::O3 => |x| x % 4 == 3,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Turn {
    First,
    Second,
}

impl Turn {
    #[inline]
    pub fn toggle(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Dash {
    y_turn: Turn,
    parity: Turn,
}

impl Dash {
    pub fn new(y_turn: Turn, parity: Turn) -> Dash {
        Dash { y_turn, parity }
    }

    fn y_turn(&self, y: u8) -> bool {
        match self.y_turn {
            Turn::First => y % 2 == 0,
            Turn::Second => y % 2 != 0,
        }
    }

    fn x_turn(&self, x: u8) -> bool {
        match self.parity {
            Turn::First => x % 2 == 0,
            Turn::Second => x % 2 != 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Variable {
    turn: Turn,
    offset: Offset,
}

impl RoadConfig {
    pub fn new(dash: Dash, reversed: bool) -> Self {
        let (offset1, offset2) = match &dash.parity {
            Turn::First => (Offset::O1, Offset::O3),
            Turn::Second => (Offset::O0, Offset::O2),
        };

        let (row1, row2) = if reversed {
            (
                Variable { turn: Turn::Second, offset: offset2 },
                Variable { turn: Turn::First, offset: offset1 },
            )
        } else {
            (
                Variable { turn: Turn::First, offset: offset1 },
                Variable { turn: Turn::Second, offset: offset2 },
            )
        };

        let turn = row1.turn.clone();

        Self { dash, row1, row2, turn }
    }

    fn set_down(&self, y: u8, x: u8, turn: &Turn) -> bool {
        if self.dash.y_turn(y) {
            self.dash.x_turn(x)
        } else if self.row1.turn == *turn {
            self.row1.offset.predicate()(x)
        } else if self.row2.turn == *turn {
            self.row2.offset.predicate()(x)
        } else {
            false
        }
    }

    pub fn plan(
        &self,
        rect: OuterRectangle,
        grid: &HashMap<RoomXY, RoomPart>,
        plan: &mut RoomPlan,
    ) {
        let occupied = plan.occupied();

        let mut roads = HashSet::new();

        let mut turn = self.turn.clone();
        let (x0, y0, x1, y1) = rect;
        for y in y0..=y1 {
            if !self.dash.y_turn(y) {
                turn = turn.toggle()
            }

            for x in x0..=x1 {
                if self.set_down(y, x, &turn) {
                    let cell = unsafe {
                        RoomXY::new(
                            RoomCoordinate::unchecked_new(x),
                            RoomCoordinate::unchecked_new(y),
                        )
                    };
                    if grid.get(&cell).is_some_and(|part| part.is_internal())
                        && !occupied.contains(&cell)
                    {
                        roads.insert(cell);
                    }
                }
            }
        }

        plan.add_cells(
            as_squares(rect, &roads, grid)
                .into_iter()
                .filter_map(|square| square.try_round()) //try rounded partially rounded squares
                .chain(
                    roads
                        .clone()
                        .into_iter()
                        .filter(move |xy| diagonal_neighbors(xy).any(|n| roads.contains(&n))),
                ) //remove not connected roads
                .map(|xy| PlannedCell::new(xy, RoomStructure::Road(0), 4, None)),
        );
    }

    fn produce_net(
        self,
        rect: OuterRectangle,
        spawn: Option<RoomXY>,
        grid: &HashMap<RoomXY, RoomPart>,
    ) -> RoadNet {
        let mut roads = HashSet::new();

        let mut turn = self.turn.clone();
        let (x0, y0, x1, y1) = rect;
        for y in y0..=y1 {
            if !self.dash.y_turn(y) {
                turn = turn.toggle()
            }

            for x in x0..=x1 {
                if self.set_down(y, x, &turn) {
                    let cell = unsafe {
                        RoomXY::new(
                            RoomCoordinate::unchecked_new(x),
                            RoomCoordinate::unchecked_new(y),
                        )
                    };
                    if grid.get(&cell).is_some_and(|part| *part == RoomPart::Green)
                        && spawn.is_none_or(|xy| cell != xy)
                    {
                        roads.insert(cell);
                    }
                }
            }
        }

        let squares = as_squares(rect, &roads, grid);
        RoadNet::new(self, roads, squares)
    }
}

fn as_squares(
    rect: OuterRectangle,
    roads: &HashSet<RoomXY>,
    grid: &HashMap<RoomXY, RoomPart>,
) -> Vec<Square> {
    let (x0, y0, x1, y1) = rect;
    let mut squares = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let cell = unsafe {
                RoomXY::new(RoomCoordinate::unchecked_new(x), RoomCoordinate::unchecked_new(y))
            };
            if !roads.contains(&cell)
                && grid.get(&cell).is_some_and(|part| *part == RoomPart::Green)
            {
                let sides: Vec<_> =
                    square_sides(&cell, 1).filter(|side| roads.contains(side)).collect();

                if !sides.is_empty() {
                    squares.push(Square::new(cell, sides));
                }
            }
        }
    }
    squares
}

pub struct RoadNet {
    pub config: RoadConfig,
    pub roads: HashSet<RoomXY>,
    pub squares: Vec<Square>,
}

impl RoadNet {
    fn new(config: RoadConfig, roads: HashSet<RoomXY>, squares: Vec<Square>) -> Self {
        Self { config, roads, squares }
    }

    pub fn rank(&self) -> usize {
        self.squares
            .iter()
            .map(|square| match square.sides.len() {
                8 => 4,
                6 | 7 => 2,
                5 => 1,
                _ => 0,
            })
            .reduce(|acc, e| acc + e)
            .unwrap_or(0)
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::rooms::constructions::tests::{sources, spawn};
//     use crate::rooms::constructions::blueprints::tests::{perimeter, grid};
//     use super::*;

//     #[test]
//     fn roads_test() {
//         let spawn = spawn();
//         let sources = sources();

//         let perimeter = perimeter(spawn, &sources);
//         let grid = grid(&perimeter);

//         let net = best_net(perimeter.rectangle(), spawn,
// &grid).expect("expect roads net created!");

//         let expected_config = RoadConfig::new(Dash::new(Turn::Second,
// Turn::Second), false);

//         assert_eq!(expected_config, net.config, "invalid config");
//         assert_eq!(55, net.roads.len(), "invalid roads len");
//         assert_eq!(17, net.squares.len(), "invalid squares len");
//     }
// }
