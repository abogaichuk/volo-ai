use log::{debug, error};
use screeps::{HasPosition, PowerType};
use crate::{movement::MovementGoal, units::{power_creep::build_goal, actions::factory_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct OperateFactory {
    next: Option<Box<dyn Action>>,
}

impl OperateFactory {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateFactory {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if let (Some(factory), Some(_), true) = (
            factory_without_effect(unit),
            unit.get_power(PowerType::OperateFactory),
            unit.is_power_enabled(PowerType::OperateFactory),
        ) {
            if unit.pos().get_range_to(factory.pos()) <= LONG_RANGE_ACTION {
                let res = unit.use_power(PowerType::OperateFactory, Some(factory));
                debug!("creep {} operate storage res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                None
            } else {
                Some(build_goal(factory.pos(), LONG_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}