use log::{debug, error};
use screeps::{HasPosition, PowerType};
use crate::{movement::MovementGoal, units::power_creep::{build_goal, controller_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct OperateController {
    next: Option<Box<dyn Action>>,
}

impl OperateController {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateController {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if controller_without_effect(unit.home_controller())
            && unit.get_power(PowerType::OperateController).is_some()
            && unit.is_power_enabled(PowerType::OperateController)
        {
            if unit.pos().get_range_to(unit.home_controller().pos()) <= LONG_RANGE_ACTION
            {
                let res = unit.use_power(PowerType::OperateController, Some(unit.home_controller()));
                debug!("creep {} operate controller res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                None
            } else {
                Some(build_goal(unit.home_controller().pos(), LONG_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}