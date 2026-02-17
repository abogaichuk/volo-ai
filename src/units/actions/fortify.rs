use screeps::{HasPosition, PowerType};

use crate::{movement::MovementGoal, units::power_creep::build_goal, utils::constants::LONG_RANGE_ACTION};

use super::{Action, PcUnit};

#[derive(Default)]
pub struct Fortify {
    next: Option<Box<dyn Action>>,
}

impl Action for Fortify {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if unit.is_power_available(PowerType::Fortify) {
            //todo take from room history
            if let Some(rampart) = unit.home_lowest_perimeter() {
                if unit.pos().in_range_to(rampart.pos(), LONG_RANGE_ACTION) {
                    //todo moving safe for powercreep
                    let _ = unit.use_power(PowerType::Fortify, Some(rampart));
                    None
                } else {
                    Some(build_goal(rampart.pos(), LONG_RANGE_ACTION, None))
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