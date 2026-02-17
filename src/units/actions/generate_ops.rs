use screeps::PowerType;
use crate::movement::MovementGoal;

use super::{Action, PcUnit};

pub struct GenerateOps {
    next: Option<Box<dyn Action>>,
}

impl GenerateOps {
    pub fn new(next: impl Action + 'static) -> Self {
        Self {
            next: Some(Box::new(next)),
        }
    }
}

impl Action for GenerateOps {
    fn handle(&self, unit: &PcUnit) -> Option<MovementGoal> {
        if unit.is_power_available(PowerType::GenerateOps) {
            let _ = unit.use_power(PowerType::GenerateOps, None);
            return None;
        } else {
            self.next().as_ref().and_then(|next| next.handle(unit))
        }
    }

    fn next(&self) -> &Option<Box<dyn Action>> {
        &self.next
    }
}