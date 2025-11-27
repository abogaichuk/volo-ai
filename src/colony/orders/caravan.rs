use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    hash::{Hash, Hasher},
};

use log::debug;
use serde::{Deserialize, Serialize};
use screeps::{game, Direction, RoomName};

use crate::{
    colony::{less_cga, prefered_room},
    commons::{capture_room_numbers, get_room_regex},
    movement::Movement,
    rooms::wrappers::claimed::Claimed,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct CaravanOrder {
    pub(crate) room: Option<RoomName>,
    pub(crate) caravan: Caravan,
    pub(crate) from: RoomName,
    pub(crate) direction: Option<Direction>,
    pub(crate) timeout: u32,
}

impl CaravanOrder {
    pub(crate) fn new(creeps: BTreeMap<String, u32>, from: RoomName) -> Self {
        Self {
            room: None,
            caravan: Caravan::new(creeps),
            from,
            direction: None,
            timeout: game::time() + 2000,
        }
    }

    fn caravan_direction(&self, current: RoomName) -> Option<Direction> {
        if self.from == current {
            None
        } else if self.from.x_coord() < current.x_coord() {
            Some(Direction::Right)
        } else if self.from.x_coord() > current.x_coord() {
            Some(Direction::Left)
        } else if self.from.y_coord() < current.y_coord() {
            Some(Direction::Bottom)
        } else {
            Some(Direction::Top)
        }
    }

    pub(crate) fn catch_caravan(
        &mut self,
        current: RoomName,
        bases: &HashMap<RoomName, Claimed>,
        movement: &Movement,
    ) -> Option<(RoomName, RoomName)> {
        if self.direction.is_none() && self.from != current {
            self.direction = self.caravan_direction(current);

            self.direction
                .and_then(|direction| {
                    let mut result: Option<_> = None;
                    let mut distance: usize = 0;
                    let mut from = current;
                    while let Some(next_room) = next_room(from, direction) {
                        debug!(
                            "catch caravan: direction: {}, next_room: {}, from: {}, distance: {}",
                            direction, next_room, from, distance
                        );
                        from = next_room;

                        if distance > 3
                            && let Some((base, range)) =
                                prefered_room(next_room, movement, bases.values(), less_cga)
                        {
                            result = match result.take() {
                                None => Some((base, next_room, range)),
                                existed => existed.map(|(prev_base, prev_ambush, prev_range)| {
                                    if prev_range > range {
                                        (base, next_room, range)
                                    } else {
                                        (prev_base, prev_ambush, prev_range)
                                    }
                                }),
                            }
                        }
                        distance += 1;
                    }
                    result
                })
                .map(|(base_name, ambush, _)| (base_name, ambush))
        } else {
            None
        }
    }
}

impl Eq for CaravanOrder {}
impl PartialEq for CaravanOrder {
    fn eq(&self, other: &CaravanOrder) -> bool {
        self.caravan == other.caravan
    }
}

impl Hash for CaravanOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.caravan.hash(state);
    }
}

impl fmt::Debug for CaravanOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CaravanOrder[base: {:?}, caravan: {:?}, from: {}, direction: {:?}]",
            self.room, self.caravan, self.from, self.direction
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Caravan {
    pub screeps: BTreeMap<String, u32>,
}

impl Caravan {
    pub fn new(creeps: BTreeMap<String, u32>) -> Self {
        Self { screeps: creeps }
    }
}

impl Eq for Caravan {}
// Only keys are used for equality
impl PartialEq for Caravan {
    fn eq(&self, other: &Self) -> bool {
        let mut self_keys: Vec<_> = self.screeps.keys().collect();
        let mut other_keys: Vec<_> = other.screeps.keys().collect();
        self_keys.sort();
        other_keys.sort();
        self_keys == other_keys
    }
}

// Only keys are used for hashing
impl Hash for Caravan {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut keys: Vec<_> = self.screeps.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
        }
    }
}

fn next_room(from: RoomName, direction: Direction) -> Option<RoomName> {
    let next = match direction {
        Direction::Right => from.checked_add((1, 0)),
        Direction::Bottom => from.checked_add((0, 1)),
        Direction::Left => from.checked_add((-1, 0)),
        Direction::Top => from.checked_add((0, -1)),
        _ => None,
    };

    let re = get_room_regex();
    next.filter(|room| {
        capture_room_numbers(&re, *room)
            .map(|(f_num, s_num)| !(f_num % 10 == 0 && s_num % 10 == 0))
            .unwrap_or(false)
    })
}
