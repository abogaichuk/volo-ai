use screeps::{HasPosition, PowerType};

use crate::{movement::MovementGoal, units::{power_creep::build_goal, actions::tower_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct OperateTower {
    next: Option<Box<dyn Action>>,
}

impl OperateTower {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateTower {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if unit.is_power_available(PowerType::OperateTower) {
            if let Some(tower) = tower_without_effect(unit) {
                if unit.pos().in_range_to(tower.pos(), LONG_RANGE_ACTION) {
                    let _ = unit.use_power(PowerType::OperateTower, Some(tower));
                    None
                } else {
                    Some(build_goal(tower.pos(), LONG_RANGE_ACTION, None))
                }
            } else {
                None
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}