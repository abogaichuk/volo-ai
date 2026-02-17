use log::{debug, error};
use screeps::{HasPosition, PowerType};
use crate::{movement::MovementGoal, units::{power_creep::build_goal, actions::spawn_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct OperateSpawn {
    next: Option<Box<dyn Action>>,
}

impl OperateSpawn {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateSpawn {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if let (Some(spawn), Some(_)) =
            (spawn_without_effect(unit), unit.get_power(PowerType::OperateSpawn))
            && unit.is_power_enabled(PowerType::OperateSpawn)
        {
            if unit.pos().get_range_to(spawn.pos()) <= LONG_RANGE_ACTION {
                let res = unit.use_power(PowerType::OperateSpawn, Some(spawn));
                debug!("creep {} operate spawn res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                None
            } else {
                Some(build_goal(spawn.pos(), LONG_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}