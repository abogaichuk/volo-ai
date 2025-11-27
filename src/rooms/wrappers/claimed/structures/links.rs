use log::*;
use screeps::{HasPosition, ResourceType};
use crate::rooms::{state::constructions::{LinkType, RoomPlan, RoomStructure}, wrappers::claimed::Claimed};

impl Claimed {
    pub(crate) fn run_links(&self, plan: Option<&RoomPlan>) {
        if let Some(plan) = plan {
            let mut sender = None;
            let mut receiver = None;
            let mut ctrl = None;
            let mut other = Vec::new();

            for cell in plan.get_links() {
                if let RoomStructure::Link(link_type) = cell.structure {
                    match link_type {
                        LinkType::Sender => sender = self.links.iter().find(|l| l.pos().xy() == cell.xy),
                        LinkType::Receiver => receiver = self.links.iter().find(|l| l.pos().xy() == cell.xy),
                        LinkType::Ctrl => ctrl = self.links.iter().find(|l| l.pos().xy() == cell.xy),
                        _ => other.extend(self.links.iter().find(|l| l.pos().xy() == cell.xy)),
                    }
                }
            }

            if let Some(farm_link) = other.iter()
                .find(|link| link.cooldown() == 0 &&
                    link.store().get_used_capacity(Some(ResourceType::Energy)) > 700)
            {
                if let Some(receiver) = receiver {
                    let transfer_result = farm_link.transfer_energy(receiver, None);
                    match transfer_result {
                        Ok(()) => {},
                        Err(err) => { warn!("room: {} farm link transfer error: {:?}", self.get_name(), err) }
                    }
                }
                //todo else -> send to ctrl_link?
            }

            if let Some(ctrl_link) = ctrl {
                if ctrl_link.store().get_free_capacity(Some(ResourceType::Energy)) > 500 &&
                    let Some(sender) = sender
                        .filter(|s| s.cooldown() == 0 && s.store().get_used_capacity(Some(ResourceType::Energy)) > 0)
                {
                    let transfer_result = sender.transfer_energy(ctrl_link, None);

                    match transfer_result {
                        Ok(()) => {},
                        Err(err) => { warn!("room: {} sender link transfer error: {:?}", self.get_name(), err) }
                    }
                }
            }
        }
    }
}
