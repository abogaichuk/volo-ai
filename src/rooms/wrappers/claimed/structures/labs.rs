use log::*;
use screeps::{HasId, ResourceType, StructureLab};
use std::{cmp, collections::HashSet};
use itertools::Itertools;
use crate::{
    commons::find_container_with, rooms::{
        RoomEvent, shelter::Labs, state::{
            RoomState, constructions::{LabStatus, RoomStructure},
            requests::{CarryData, Request, RequestKind, assignment::Assignment, meta::Status}
        }, wrappers::claimed::Claimed
    }
};


const MIN_ENERGY_AMOUNT: u32 = 1000;
const MIN_RESOURCE_AMOUNT: u32 = 2000;
const LAB_PRODUCTION: u32 = 5;

impl Claimed {
    // pub(crate) fn run_labs(
    //     &self,
    //     requests: &HashSet<Request>,
    //     room_memory: &RoomState) -> impl Iterator<Item = RoomEvent>
    // {
    //     debug!("{} running labs", self.get_name());
    //     None.into_iter()
    //     //update lab statuses to current boosts
    //     // let mut out = self.update_lab_state(room_memory);
    //     // if !out.is_empty() {
    //     //     return out.into_iter();
    //     // }

    //     // let Some(storage) = self.storage() else {
    //     //     return out.into_iter();
    //     // };
    //     // //split labs by boost or production purposes
    //     // let (for_boost, others):(_, Vec<_>) = self.labs.iter()
    //     //     .map(|lab| {
    //     //         let boost_resource = room_memory.labs.get(&lab.id())
    //     //             .and_then(|status| {
    //     //                 match status {
    //     //                     LabStatus::Boost(resource) => Some(*resource),
    //     //                     _ => None
    //     //                 }
    //     //             });
    //     //         (lab, boost_resource)
    //     //     })
    //     //     .partition(|(_, res)| res.is_some());

    //     // // keep boost labs ready to boost by creating carry requests
    //     // let boost_events = for_boost.into_iter()
    //     //     .flat_map(|(lab, resource)|
    //     //         self.keep_boost_ready(lab, resource.expect("expect boosted resources")));
    //     // out.extend(boost_events);


    //     // let in_progress = requests.iter()
    //     //     .any(|r| matches!(r.kind, RequestKind::Lab(_)) &&
    //     //         matches!(r.status(), Status::InProgress | Status::OnHold));

    //     // if !in_progress {
    //     //     if let Some(mut request) = new_request(requests) {
    //     //         if let RequestKind::Lab(data) = &request.kind {
    //     //             let events: Vec<RoomEvent> = data.resource.reaction_components()
    //     //                 .iter()
    //     //                 .flat_map(|components| components.iter()
    //     //                     .filter_map(|component| {
    //     //                         let capacity = storage.store().get_used_capacity(Some(*component));
    //     //                         if capacity < LAB_PRODUCTION {
    //     //                             Some(RoomEvent::Lack(*component, data.amount - capacity))
    //     //                         } else {
    //     //                             None
    //     //                         }
    //     //                     }))
    //     //                     .collect();

    //     //             if events.is_empty() {
    //     //                 request.join(None, None);
    //     //                 out.push(RoomEvent::ReplaceRequest(request));
    //     //             } else {
    //     //                 out.extend(events);
    //     //             }
    //     //         } else {
    //     //             error!("{} incorrect request kind: {:?}", self.get_name(), request);
    //     //         }
    //     //     } else {
    //     //         out.extend(others.into_iter()
    //     //             .filter_map(|(lab, _)| self.unload(lab, &[])));
    //     //     }
    //     // }

    //     // out.into_iter()
    // }
    pub(crate) fn run_labs(
        &self,
        requests: &HashSet<Request>,
        labs: Labs,
        room_memory: &RoomState) -> impl Iterator<Item = RoomEvent>
    {
        debug!("{} running labs", self.get_name());
        //update lab statuses to current boosts
        let mut out = self.update_lab_state(room_memory);
        if !out.is_empty() {
            return out.into_iter();
        }

        let Some(storage) = self.storage() else {
            return out.into_iter();
        };

        //split labs by boost or production purposes
        let Labs { inputs, outputs, boosts } = labs;

        // let (for_boost, others):(_, Vec<_>) = self.labs.iter()
        //     .map(|lab| {
        //         let boost_resource = room_memory.labs.get(&lab.id())
        //             .and_then(|status| {
        //                 match status {
        //                     LabStatus::Boost(resource) => Some(*resource),
        //                     _ => None
        //                 }
        //             });
        //         (lab, boost_resource)
        //     })
        //     .partition(|(_, res)| res.is_some());




        // keep boost labs ready to boost by creating carry requests
        let boost_events = boosts.into_iter()
            .flat_map(|(lab, resource)| self.keep_boost_ready(lab, resource));
        out.extend(boost_events);


        let in_progress = requests.iter()
            .any(|r| matches!(r.kind, RequestKind::Lab(_)) &&
                matches!(r.status(), Status::InProgress | Status::OnHold));

        if !in_progress {
            if let Some(mut request) = new_request(requests) {
                if let RequestKind::Lab(data) = &request.kind {
                    let events: Vec<RoomEvent> = data.resource.reaction_components()
                        .iter()
                        .flat_map(|components| components.iter()
                            .filter_map(|component| {
                                let capacity = storage.store().get_used_capacity(Some(*component));
                                if capacity < LAB_PRODUCTION {
                                    Some(RoomEvent::Lack(*component, data.amount - capacity))
                                } else {
                                    None
                                }
                            }))
                            .collect();

                    if events.is_empty() {
                        request.join(None, None);
                        out.push(RoomEvent::ReplaceRequest(request));
                    } else {
                        out.extend(events);
                    }
                } else {
                    error!("{} incorrect request kind: {:?}", self.get_name(), request);
                }
            } else {
                out.extend(inputs.into_iter().chain(outputs.into_iter())
                    .filter_map(|lab| self.unload(lab, &[])));
            }
        }

        out.into_iter()
    }

    fn update_lab_state(&self, room_memory: &RoomState) -> Vec<RoomEvent> {
        if let Some(plan) = room_memory.plan.as_ref() {
            //all unique resources needed for boosts
            let mut boost_resources: Vec<ResourceType> = room_memory.boosts.iter()
                .flat_map(|boost_reason| boost_reason.0.value())
                .unique()
                .collect();

            let in_use:HashSet<ResourceType> = plan.boosts_in_use();

            plan.get_labs()
                .filter_map(|cell| match cell.structure {
                    RoomStructure::Lab(status) => {
                        match status {
                            LabStatus::Boost(resource) => {
                                (!boost_resources.contains(&resource))
                                    .then(|| {
                                        let mut new_cell = cell.clone();
                                        new_cell.structure = RoomStructure::Lab(LabStatus::Output);
                                        RoomEvent::ReplaceCell(new_cell)
                                    })
                            }
                            LabStatus::Output if !boost_resources.is_empty() => {
                                let resource = boost_resources.swap_remove(0);
                                (!in_use.contains(&resource))
                                    .then(|| {
                                        let mut new_cell = cell.clone();
                                        new_cell.structure = RoomStructure::Lab(LabStatus::Boost(resource));
                                        RoomEvent::ReplaceCell(new_cell)
                                    })
                            }
                            _ => None
                        }
                    }
                    _ => { None }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    // fn update_lab_state(&self, room_memory: &RoomState) -> Vec<RoomEvent> {
    //     //all unique resources needed for boosts
    //     let mut boost_resources: Vec<ResourceType> = room_memory.boosts.iter()
    //         .flat_map(|boost_reason| boost_reason.0.value())
    //         .unique()
    //         .collect();

    //     let in_use:HashSet<ResourceType> = room_memory.labs.iter()
    //         .filter_map(|(_, state)| {
    //             match state {
    //                 LabStatus::Boost(resource) => Some(*resource),
    //                 _ => None
    //             }
    //         })
    //         .collect();

    //     self.labs.iter()
    //         .filter_map(|lab| match room_memory.labs.get(&lab.id()) {
    //             None => Some(RoomEvent::UpdateLab(lab.id(), LabStatus::Output)),
    //             Some(LabStatus::Boost(resource)) if !boost_resources.contains(resource) => {
    //                 if !boost_resources.contains(resource) {
    //                     Some(RoomEvent::UpdateLab(lab.id(), LabStatus::Output))
    //                 } else {
    //                     None
    //                 }
    //             },
    //             Some(LabStatus::Output) if !boost_resources.is_empty() => {
    //                 let resource = boost_resources.swap_remove(0);
    //                 if !in_use.contains(&resource) {
    //                     Some(RoomEvent::UpdateLab(lab.id(), LabStatus::Boost(resource)))
    //                 } else {
    //                     None
    //                 }
    //             },
    //             _ => None
    //         })
    //         .collect()
    // }

    fn keep_boost_ready(&self, lab: &StructureLab, resource: ResourceType) -> impl Iterator<Item = RoomEvent> {
        let supply_energy = self.load_lab(lab, (ResourceType::Energy, MIN_ENERGY_AMOUNT));
        let supply_resource = self.unload(lab, &[resource])
            .or_else(|| self.load_lab(lab, (resource, MIN_RESOURCE_AMOUNT)));

        supply_energy.into_iter().chain(supply_resource)
    }

    pub fn load_lab(&self, lab: &StructureLab, component: (ResourceType, u32)) -> Option<RoomEvent> {
        if lab.store().get_used_capacity(Some(component.0)) < component.1 {
            find_container_with(component.0, None, self.storage(), self.terminal(), self.factory())
                .map(|(id, amount)| {
                    let min_amount = cmp::min(amount, component.1);
                    RoomEvent::Request(Request::new(
                        RequestKind::Carry(CarryData::new(
                            id,
                            lab.raw_id(),
                            component.0,
                            min_amount)),
                        Assignment::Single(None)))
                })
        } else {
            None
        }
    }
}

fn new_request(requests: &HashSet<Request>) -> Option<Request> {
    requests.iter()
        .find(|r| matches!(r.kind, RequestKind::Lab(_)) &&
            matches!(r.status(), Status::Created))
        .cloned()
}

// fn get_request(requests: &HashSet<Request>) -> Option<Request> {
//     let active = requests.iter()
//         .any(|r| matches!(r.kind, RequestKind::Lab(_)) &&
//             matches!(r.status(), Status::InProgress | Status::OnHold));

//     (!active)
//         .then(|| requests.iter()
//             .find(|r| matches!(r.kind, RequestKind::Lab(_)) &&
//                 matches!(r.status(), Status::Created))
//             .cloned())?
// }