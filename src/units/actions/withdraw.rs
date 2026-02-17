use std::sync::Arc;

use log::warn;
use screeps::{ResourceType, HasPosition};

use crate::{movement::MovementGoal, units::{actions::ActionFn, power_creep::build_goal}, utils::constants::CLOSE_RANGE_ACTION};

use super::{Action, PcUnit};

pub struct Withdraw {
    next: Option<Box<dyn Action>>,
    amount: u32
}

impl Withdraw {
    pub fn new(next: impl Action + 'static, amount: u32) -> Self {
        Self {
            next: Some(Box::new(next)),
            amount
        }
    }
}

impl Action for Withdraw {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if unit.used_capacity(Some(ResourceType::Ops)) < self.amount {
            if let Some(storage) = unit.home_storage()
                && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= self.amount
            {
                if unit.pos().is_near_to(storage.pos()) {
                    let _ = unit.withdraw(storage, ResourceType::Ops, None);
                    None
                } else {
                    Some(build_goal(storage.pos(), CLOSE_RANGE_ACTION, None))
                }
            } else {
                warn!("room: {} resource ops not enough!!", unit.home_name());
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


// fn withdraw(amount: i32, next: ActionFn) -> ActionFn {
//     Arc::new(move |unit| {
//         if unit.used_capacity(Some(ResourceType::Ops)) < amount {
//             if let Some(storage) = unit.home_storage()
//                 && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= amount
//             {
//                 if unit.pos().is_near_to(storage.pos()) {
//                     let _ = unit.withdraw(storage, ResourceType::Ops, None);
//                     return None;
//                 } else {
//                     return Some(build_goal(storage.pos(), CLOSE_RANGE_ACTION, None));
//                 }
//             } else {
//                 warn!("room: {} resource ops not enough!!", unit.home_name());
//                 return None;
//             }
//         }
//         next(unit)
//     })
// }