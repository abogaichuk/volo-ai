use std::cmp::Ordering;
use std::collections::HashMap;
use std::str::FromStr;

use itertools::Itertools;
use log::*;
use screeps::pathfinder::SearchGoal;
use screeps::{HasPosition, Position, RoomName, RoomXY, game};

use super::{PlannedCell, RoomPart, RoomPlan, RoomPlannerError, RoomStructure};
use crate::commons::is_cpu_on_low;
use crate::movement::callback::{closest_multi_rooms_range, construction_multi_rooms};
use crate::movement::{find_many, find_path};
use crate::rooms::is_extractor;
use crate::rooms::wrappers::farm::Farm;

const STEP: usize = 5;

#[derive(Debug, Clone)]
struct CostedRoute {
    distance: usize,
    path: Vec<Position>,
}

impl PartialOrd for CostedRoute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CostedRoute {
    fn cmp(&self, other: &Self) -> Ordering {
        let diff = self.distance.abs_diff(other.distance);

        match self.distance.cmp(&other.distance) {
            Ordering::Greater => match (self.path.len() + diff).cmp(&other.path.len()) {
                Ordering::Less => Ordering::Less,
                _ => Ordering::Greater,
            },
            Ordering::Less => match self.path.len().cmp(&(other.path.len() + diff)) {
                Ordering::Less => Ordering::Less,
                _ => Ordering::Greater,
            },
            Ordering::Equal => self.path.len().cmp(&other.path.len()),
        }
    }
}

impl PartialEq for CostedRoute {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.path.len() == other.path.len()
    }
}

impl Eq for CostedRoute {}

impl Farm {
    pub fn plan_room(
        &self,
        existed_plans: HashMap<RoomName, &RoomPlan>,
    ) -> Result<HashMap<RoomName, RoomPlan>, RoomPlannerError> {
        let targets: Vec<Position> = self.plan_targets();

        let mut existed_roads: HashMap<Position, usize> =
            existed_plans.iter().flat_map(|(name, plan)| plan.costed_roads(*name)).collect();

        let mut structures: HashMap<RoomName, Vec<RoomXY>> = existed_plans
            .iter()
            .map(|(name, plan)| (*name, plan.unwalkable_structures()))
            .collect();

        let base_internal = *existed_roads
            .iter()
            .find_map(|(pos, distance)| if *distance == 0 { Some(pos) } else { None })
            .ok_or(RoomPlannerError::RoadConnectionFailure)?;

        for (target, _) in targets
            .into_iter()
            .map(|target| {
                let search_res = find_path(target, base_internal, 1, closest_multi_rooms_range());
                (target, if search_res.incomplete() { usize::MAX } else { search_res.path().len() })
            })
            .sorted_by_key(|(_, distance)| *distance)
        {
            let max_distance = existed_roads.values().max().copied().unwrap_or_default();

            let mut max = STEP;
            let mut min = 0;
            let mut best_route: Option<CostedRoute> = None;

            let cpu_start = game::cpu::get_used();
            while min < max_distance {
                if is_cpu_on_low() {
                    return Err(RoomPlannerError::LowCPU);
                }

                let goals = existed_roads.iter().filter_map(|(pos, cost)| {
                    if *cost <= max && *cost >= min { Some(SearchGoal::new(*pos, 0)) } else { None }
                });

                let search_result = find_many(target, goals, construction_multi_rooms(&structures));

                if search_result.incomplete() {
                    error!("{} construction search_result incomplete!", self.get_name());
                    min = min.saturating_add(STEP);
                    max = max.saturating_add(STEP);
                    continue;
                }

                let mut path = search_result.path().into_iter().filter(|pos| !pos.is_room_edge());
                let distance = path
                    .next_back()
                    .and_then(|pos| existed_roads.get(&pos))
                    .ok_or(RoomPlannerError::RoadPlanFailure)?;

                let route = CostedRoute { distance: *distance, path: path.collect() };
                let best_distance = best_route.as_ref().map(|b| b.distance);

                let (shortest, new_min, new_max) = if let Some(costed_route) = best_route.take() {
                    debug!(
                        "comparing distance: {}, len: {}, last: {:?}, with current distance: {}, len: {}, last: {:?}",
                        distance,
                        route.path.len(),
                        route.path.last().map(|pos| (pos.x().u8(), pos.y().u8())),
                        costed_route.distance,
                        costed_route.path.len(),
                        costed_route.path.last().map(|pos| (pos.x().u8(), pos.y().u8()))
                    );
                    match costed_route.cmp(&route) {
                        Ordering::Less => {
                            debug!(
                                "less case: min: {}, max: {}, costed_route.dist: {}",
                                min, max, costed_route.distance
                            );
                            (costed_route, min.saturating_add(STEP), max.saturating_add(STEP))
                        }
                        Ordering::Equal => {
                            debug!(
                                "equal case: min: {}, max: {}, costed_route.dist: {}",
                                min, max, costed_route.distance
                            );
                            (costed_route, min.saturating_add(STEP), max.saturating_add(STEP))
                        }
                        Ordering::Greater => {
                            debug!(
                                "greater case: min: {}, max: {}, route.dist: {}",
                                min, max, route.distance
                            );
                            (route, min.saturating_add(STEP), max.saturating_add(STEP))
                        }
                    }
                } else {
                    debug!("None case: min: {}, max: {}, route.dist: {}", min, max, route.distance);
                    (route, min.saturating_add(STEP), max.saturating_add(STEP))
                };

                let cpu_used = game::cpu::get_used() - cpu_start;
                info!(
                    "{} while cpu used: {}, new_min: {}, new_max: {}, shortest:[distance: {}, len:{}, last:{:?}], best_distance: {:?}",
                    self.get_name(),
                    cpu_used,
                    new_min,
                    new_max,
                    shortest.distance,
                    shortest.path.len(),
                    shortest.path.last().map(|pos| (pos.x().u8(), pos.y().u8())),
                    best_distance
                );

                min = new_min;
                max = new_max;
                best_route = Some(shortest);
            }

            if let Some(costed_route) = best_route.take() {
                info!(
                    "from: {} - distance = {} + {}",
                    target,
                    costed_route.distance,
                    costed_route.path.len() - 1
                );
                let r = costed_route.path.iter().fold(
                    String::from_str("").expect("expect str"),
                    |acc, elem| {
                        format!(
                            "{} [{}: {}, {}]",
                            acc,
                            elem.room_name(),
                            elem.x().u8(),
                            elem.y().u8()
                        )
                    },
                );
                info!("route: {}", r);

                let mut path = costed_route.path.into_iter();
                let container = path.next().ok_or(RoomPlannerError::RoadPlanFailure)?;
                structures
                    .entry(container.room_name())
                    .and_modify(|s| s.push(container.xy()))
                    .or_insert_with(|| vec![container.xy()]);

                for (i, pos) in path.rev().enumerate() {
                    existed_roads.insert(pos, costed_route.distance + i + 1);
                }
            } else {
                return Err(RoomPlannerError::RoadPlanFailure);
            }
        }

        let mut plans: HashMap<RoomName, RoomPlan> =
            existed_roads.into_iter().fold(HashMap::new(), |mut acc, (pos, cost)| {
                acc.entry(pos.room_name()).or_default().add_cell(PlannedCell::new(
                    pos.xy(),
                    RoomStructure::Road(cost),
                    0,
                    None,
                ));
                acc
            });

        for xy in structures
            .get(&self.get_name())
            .ok_or(RoomPlannerError::ContainerPlacementError)?
            .iter()
        {
            let cell = PlannedCell::new(*xy, RoomStructure::Container(RoomPart::Red), 0, None);
            plans.entry(self.get_name()).and_modify(|plan| plan.add_cell(cell));
        }

        Ok(plans)
    }

    fn plan_targets(&self) -> Vec<Position> {
        self.sources
            .iter()
            .map(|source| source.pos())
            .chain(
                self.mineral
                    .as_ref()
                    .filter(|mineral| is_extractor(mineral))
                    .map(|mineral| mineral.pos()),
            )
            .collect()
    }
}
