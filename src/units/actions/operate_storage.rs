use log::{debug, error};
use screeps::{HasPosition, PowerType};
use crate::{movement::MovementGoal, units::{power_creep::build_goal, actions::full_storage_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct OperateStorage {
    next: Option<Box<dyn Action>>,
}

impl OperateStorage {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateStorage {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if let (Some(storage), Some(_)) =
            (full_storage_without_effect(unit), unit.get_power(PowerType::OperateStorage))
        {
            if unit.pos().get_range_to(storage.pos()) <= LONG_RANGE_ACTION {
                let res = unit.use_power(PowerType::OperateStorage, Some(storage));
                debug!("creep {} operate storage res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                None
            } else {
                Some(build_goal(storage.pos(), LONG_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}