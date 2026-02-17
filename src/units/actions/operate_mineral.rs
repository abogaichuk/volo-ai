use log::error;
use screeps::{HasPosition, PowerType};
use crate::{movement::MovementGoal, units::{power_creep::build_goal, actions::mineral_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

#[derive(Default)]
pub struct OperateMineral {
    next: Option<Box<dyn Action>>,
}

impl OperateMineral {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateMineral {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if mineral_without_effect(unit) && unit.get_power(PowerType::RegenMineral).is_some()
        {
            if unit.pos().in_range_to(unit.home_mineral().pos(), 3) {
                let res = unit.use_power(PowerType::RegenMineral, Some(unit.home_mineral()));
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                None
            } else {
                Some(build_goal(unit.home_mineral().pos(), LONG_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}