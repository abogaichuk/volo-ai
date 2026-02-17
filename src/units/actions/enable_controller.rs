use std::sync::Arc;

use screeps::HasPosition;
use crate::{movement::MovementGoal, units::{actions::ActionFn, power_creep::build_goal}, utils::constants::CLOSE_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct EnableController {
    next: Option<Box<dyn Action>>,
}

impl EnableController {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for EnableController {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        let controller = unit.home_controller();

        if !controller.is_power_enabled()
        {
            if unit.pos().is_near_to(controller.pos()) {
                let _ = unit.enable_room(controller);
                None
            } else {
                Some(build_goal(controller.pos(), CLOSE_RANGE_ACTION, None))
            }
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}

// fn enable_controller(next: ActionFn) -> ActionFn {
//     Arc::new(move |unit| {
//         let controller = unit.home_controller();

//         if !controller.is_power_enabled()
//         {
//             if unit.pos().is_near_to(controller.pos()) {
//                 let _ = unit.enable_room(controller);
//                 None
//             } else {
//                 Some(build_goal(controller.pos(), CLOSE_RANGE_ACTION, None))
//             }
//         }
//         // otherwise, delegate
//         next(unit)
//     })
// }