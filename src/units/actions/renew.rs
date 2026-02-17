use log::warn;
use screeps::HasPosition;
use crate::{movement::MovementGoal, units::power_creep::build_goal, utils::constants::CLOSE_RANGE_ACTION};

use super::{Action, PcUnit};


const MIN_TICKS: u32 = 100;

pub struct Renew {
    next: Option<Box<dyn Action>>,
}

impl Renew {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for Renew {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if unit.ticks_to_live().is_some_and(|ticks| ticks < MIN_TICKS)
        {
            if let Some(power_spawn) = unit.home_power_spawn() {
                if unit.pos().is_near_to(power_spawn.pos()) {
                    let _ = unit.renew(power_spawn);
                    None
                } else {
                    Some(build_goal(power_spawn.pos(), CLOSE_RANGE_ACTION, None))
                }
            } else {
                warn!("power_creep: {} no powerspawn found for renew!!", unit.name());
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