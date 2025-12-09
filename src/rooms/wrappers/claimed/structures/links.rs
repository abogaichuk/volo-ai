use log::*;
use screeps::{HasPosition, ResourceType, StructureLink};

use crate::rooms::state::constructions::{LinkType, PlannedCell, RoomPlan, RoomStructure};
use crate::rooms::wrappers::claimed::Claimed;

impl Claimed {
    pub(crate) fn run_links(&self) {
        if let Some(farm_link) = self.links.other.iter().find(|link| {
            link.cooldown() == 0 && link.store().get_used_capacity(Some(ResourceType::Energy)) > 700
        })
        {
            if let Some(receiver) = self.links.receiver() {
                let transfer_result = farm_link.transfer_energy(receiver, None);
                match transfer_result {
                    Ok(()) => {}
                    Err(err) => {
                        warn!("room: {} farm link transfer error: {:?}", self.get_name(), err)
                    }
                }
            } else {
                //todo else -> send to ctrl_link?
            }
        }

        if let Some(ctrl_link) = self.links.ctrl() && ctrl_link.store().get_free_capacity(Some(ResourceType::Energy)) > 500
            && let Some(sender) = self.links.sender().filter(|s| {
                s.cooldown() == 0 && s.store().get_used_capacity(Some(ResourceType::Energy)) > 0
            })
        {
            let transfer_result = sender.transfer_energy(ctrl_link, None);
            match transfer_result {
                Ok(()) => {}
                Err(err) => {
                    warn!("room: {} sender link transfer error: {:?}", self.get_name(), err)
                }
            }
        }
    }
}

#[derive(Default)]
pub struct Links {
    sender: Option<StructureLink>,
    receiver: Option<StructureLink>,
    ctrl: Option<StructureLink>,
    other: Vec<StructureLink>,
}

impl Links {
    pub fn new(links: Vec<StructureLink>, plan: Option<&RoomPlan>) -> Self {
        let Some(plan) = plan else {
            return Links::default();
        };

        let mut sender = None;
        let mut receiver = None;
        let mut ctrl = None;
        let mut other = Vec::new();

        for link in links {
            //doesn't matter which type of link is passed for seacrh
            //hash works for xy and hight level room structure type only
            let cell =
                PlannedCell::searchable(link.pos().xy(), RoomStructure::Link(LinkType::Source));

            if let Some(planned_cell) = plan.get_cell(cell) {
                match planned_cell.structure {
                    RoomStructure::Link(link_type) => match link_type {
                        LinkType::Sender => sender = Some(link),
                        LinkType::Receiver => receiver = Some(link),
                        LinkType::Ctrl => ctrl = Some(link),
                        LinkType::Source => other.push(link),
                    },
                    _ => {
                        warn!("invalid plan search for: {}", link.pos());
                    }
                }
            }
        }

        Self { sender, receiver, ctrl, other }
    }

    pub fn sender(&self) -> Option<&StructureLink> {
        self.sender.as_ref()
    }

    pub fn receiver(&self) -> Option<&StructureLink> {
        self.receiver.as_ref()
    }

    pub fn ctrl(&self) -> Option<&StructureLink> {
        self.ctrl.as_ref()
    }
}
