use log::warn;
use screeps::{ResourceType, HasPosition};

use crate::{movement::MovementGoal, units::power_creep::build_goal, utils::constants::{CLOSE_RANGE_ACTION, MIN_STORAGE_FREE_CAPACITY}};

use super::{Action, PcUnit};

#[derive(Default)]
pub struct Transfer {
    next: Option<Box<dyn Action>>
}

impl Action for Transfer {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if unit.used_capacity(Some(ResourceType::Ops)) > unit.capacity() / 2
        {
            if let Some(storage) = unit.home_storage()
                && storage.store().get_free_capacity(None) > MIN_STORAGE_FREE_CAPACITY
            {
                if unit.pos().is_near_to(storage.pos()) {
                    let _ = unit.transfer(storage, ResourceType::Ops, None);
                    None
                } else {
                    Some(build_goal(storage.pos(), CLOSE_RANGE_ACTION, None))
                }
            } else {
                warn!("room: {}, storage is full!!", unit.home_name());
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