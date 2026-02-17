use log::error;
use screeps::{HasPosition, PowerType};
use crate::{movement::MovementGoal, units::{power_creep::build_goal, actions::source_without_effect}, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct OperateSource {
    next: Option<Box<dyn Action>>,
}

impl OperateSource {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for OperateSource {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if let (Some(source), Some(_)) =
            (source_without_effect(unit), unit.get_power(PowerType::RegenSource))
        {
            if unit.pos().in_range_to(source.pos(), LONG_RANGE_ACTION) {
                let res = unit.use_power(PowerType::RegenSource, Some(source));
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                None
            } else {
                Some(build_goal(source.pos(), LONG_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}