use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use log::{debug, error, info};
use screeps::{Position, RoomName, RoomXY, game, pathfinder::SearchGoal};

use super::{PlannedCell, RoomPart, RoomPlan, RoomPlannerError, RoomStructure};
use crate::commons::is_cpu_on_low;
use crate::movement::{callback::construction_multi_rooms, find_many};
use crate::rooms::{is_extractor, wrappers::farm::Farm};

#[derive(Debug, Clone)]
struct CostedRoute {
    remoted: usize,
    path: Vec<Position>,
}

impl CostedRoute {
    fn distance_to_safe(&self) -> usize {
        self.remoted + self.path.len()
    }
}

impl Display for CostedRoute {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "remoted from safe zone: {}, path.len(): {}, sum: {}, last: {:?}",
            self.remoted,
            self.path.len(),
            self.remoted + self.path.len(),
            self.path.last().map(|pos| (pos.x().u8(), pos.y().u8()))
        )
    }
}

impl PartialOrd for CostedRoute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CostedRoute {
    fn cmp(&self, other: &Self) -> Ordering {
        let distance = self.distance_to_safe();
        let o_distance = other.distance_to_safe();

        // let diff = distance.abs_diff(o_distance);
        match distance.cmp(&o_distance) {
            Ordering::Equal => self.path.len().cmp(&other.path.len()),
            r => r,
        }
    }
}

impl PartialEq for CostedRoute {
    fn eq(&self, other: &Self) -> bool {
        self.remoted == other.remoted && self.path.len() == other.path.len()
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

        for target in targets {
            let mut best_route: Option<CostedRoute> = None;
            let mut distance_to_safe_zone =
                existed_roads.values().max().copied().unwrap_or_default();

            let cpu_start = game::cpu::get_used();
            while distance_to_safe_zone > 0 {
                if is_cpu_on_low() {
                    return Err(RoomPlannerError::LowCPU);
                }

                let goals = existed_roads.iter().filter_map(|(pos, cost)| {
                    if *cost == distance_to_safe_zone {
                        Some(SearchGoal::new(*pos, 1))
                    } else {
                        None
                    }
                });

                let search_result = find_many(target, goals, construction_multi_rooms(&structures));

                if search_result.incomplete() || search_result.path().is_empty() {
                    error!("{} construction search_result incomplete!", self.get_name());
                    distance_to_safe_zone = distance_to_safe_zone.saturating_sub(1);
                    continue;
                }

                let path = search_result.path().into_iter().filter(|pos| !pos.is_room_edge());

                let route = CostedRoute { remoted: distance_to_safe_zone, path: path.collect() };
                let shortest = if let Some(costed_route) = best_route.take() {
                    if costed_route <= route { costed_route } else { route }
                } else {
                    route
                };

                let cpu_used = game::cpu::get_used() - cpu_start;
                debug!(
                    "{} while cpu used: {}, distance_to_safe_zone: {}, shortest: {}",
                    self.get_name(),
                    cpu_used,
                    distance_to_safe_zone,
                    shortest
                );
                distance_to_safe_zone = distance_to_safe_zone.saturating_sub(1);
                best_route = Some(shortest);
            }

            if let Some(costed_route) = best_route.take() {
                info!(
                    "from: {} - distance = {} + {}",
                    target,
                    costed_route.remoted,
                    costed_route.path.len()
                );
                let r = costed_route.path.iter().fold(String::new(), |acc, elem| {
                    format!("{} [{}: {}, {}]", acc, elem.room_name(), elem.x().u8(), elem.y().u8())
                });
                info!("route: {}", r);

                let mut path = costed_route.path.into_iter();
                let container = path.next().ok_or(RoomPlannerError::RoadPlanFailure)?;
                info!("container at: {}", container);
                structures
                    .entry(container.room_name())
                    .and_modify(|s| s.push(container.xy()))
                    .or_insert_with(|| vec![container.xy()]);

                for (i, pos) in path.rev().enumerate() {
                    existed_roads.insert(pos, costed_route.remoted + i + 1);
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

        for xy in
            structures.get(&self.get_name()).ok_or(RoomPlannerError::ContainerPlacementError)?
        {
            let cell = PlannedCell::new(*xy, RoomStructure::Container(RoomPart::Red), 0, None);
            plans.entry(self.get_name()).and_modify(|plan| plan.add_cell(cell));
        }

        Ok(plans)
    }

    fn plan_targets(&self) -> Vec<Position> {
        self.sources
            .iter()
            .map(screeps::HasPosition::pos)
            .chain(
                self.mineral
                    .as_ref()
                    .filter(|mineral| is_extractor(mineral))
                    .map(screeps::HasPosition::pos),
            )
            .collect()
    }
}
