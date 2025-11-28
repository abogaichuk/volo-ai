use log::*;
use screeps::{HasId, HasPosition, ObjectId, ResourceType, StructureLab};
use std::{cmp, collections::{HashMap, HashSet}};
use itertools::Itertools;
use crate::{
    commons::find_container_with, rooms::{
        RoomEvent, state::{
            BoostReason, constructions::{LabStatus, PlannedCell, RoomPlan, RoomStructure}, requests::{CarryData, Request, RequestKind, assignment::Assignment, meta::Status}
        }, wrappers::claimed::Claimed
    }
};

const MIN_ENERGY_AMOUNT: u32 = 1000;
const MIN_RESOURCE_AMOUNT: u32 = 2000;
const LAB_PRODUCTION: u32 = 5;

impl Claimed {
    pub(crate) fn run_labs(
        &self,
        requests: &HashSet<Request>,
        boosts: &HashMap<BoostReason, u32>
    ) -> impl Iterator<Item = RoomEvent> {
        debug!("{} running labs", self.get_name());
        //update lab statuses to current boosts
        let mut out = Vec::new();

        let Some(storage) = self.storage() else {
            return out.into_iter();
        };

        if let Some(event) = self.update_lab_state(boosts) {
            out.push(event);
            return out.into_iter();
        }

        let mut out = Vec::new();
        // keep boost labs ready to boost by creating carry requests
        let boost_events = self.labs.boosts().iter()
            .flat_map(|(res, lab)| self.keep_boost_ready(res, lab));
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
                                (capacity < LAB_PRODUCTION)
                                    .then(|| RoomEvent::Lack(*component, data.amount - capacity))
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
                out.extend(self.labs.inputs().iter()
                    .chain(self.labs.outputs().iter())
                    .filter_map(|lab| self.unload(lab, &[])));
            }
        }

        out.into_iter()
    }

    fn update_lab_state(&self, boosts: &HashMap<BoostReason, u32>) -> Option<RoomEvent> {
        //all unique boostable resources
        let boost_resources: Vec<ResourceType> = boosts.iter()
            .flat_map(|boost_reason| boost_reason.0.value())
            .unique()
            .collect();

        boost_resources.iter()
            .find_map(|res| (!self.labs.boosts().contains_key(res))
                .then(|| self.labs.outputs.first()
                    .map(|lab| {
                        let cell = PlannedCell::searchable(
                            lab.pos().xy(),
                            RoomStructure::Lab(LabStatus::Boost(*res)));
                        RoomEvent::ReplaceCell(cell)
                    })))
            .flatten()
            .or_else(|| self.labs.boosts.iter()
                .find_map(|(res, lab)| (!boost_resources.contains(res))
                    .then(|| {
                        let cell = PlannedCell::searchable(
                            lab.pos().xy(),
                            RoomStructure::Lab(LabStatus::Output));
                        RoomEvent::ReplaceCell(cell)
                    })))
    }

    fn keep_boost_ready(&self, resource: &ResourceType, lab: &StructureLab) -> impl Iterator<Item = RoomEvent> {
        let supply_energy = self.load_lab(lab, (ResourceType::Energy, MIN_ENERGY_AMOUNT));
        let supply_resource = self.unload(lab, &[*resource])
            .or_else(|| self.load_lab(lab, (*resource, MIN_RESOURCE_AMOUNT)));

        supply_energy.into_iter().chain(supply_resource)
    }

    pub fn load_lab(&self, lab: &StructureLab, component: (ResourceType, u32)) -> Option<RoomEvent> {
        (lab.store().get_used_capacity(Some(component.0)) < component.1)
            .then(|| find_container_with(component.0, None, self.storage(), self.terminal(), self.factory())
                .map(|(id, amount)| {
                    let min_amount = cmp::min(amount, component.1);
                    RoomEvent::Request(Request::new(
                        RequestKind::Carry(CarryData::new(
                            id,
                            lab.raw_id(),
                            component.0,
                            min_amount)),
                        Assignment::Single(None)))
                }))
            .flatten()
    }
}

fn new_request(requests: &HashSet<Request>) -> Option<Request> {
    requests.iter()
        .find(|r| matches!(r.kind, RequestKind::Lab(_)) &&
            matches!(r.status(), Status::Created))
        .cloned()
}

#[derive(Default)]
pub(crate) struct Labs {
    inputs: Vec<StructureLab>,
    outputs: Vec<StructureLab>,
    boosts: HashMap<ResourceType, StructureLab>
}

impl Labs {
    pub fn new(labs: Vec<StructureLab>, plan: Option<&RoomPlan>) -> Self {
        let Some(plan) = plan else {
            return Labs::default();
        };

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut boosts = HashMap::new();

        //split labs by boost or production purposes
        for lab in labs.into_iter() {
            let cell = PlannedCell::searchable(
                    lab.pos().xy(),
                    RoomStructure::Lab(LabStatus::Output));
            if let Some(planned_cell) = plan.get_cell(cell) {
                match planned_cell.structure {
                    RoomStructure::Lab(LabStatus::Input) => inputs.push(lab),
                    RoomStructure::Lab(LabStatus::Output) => outputs.push(lab),
                    RoomStructure::Lab(LabStatus::Boost(r)) => { boosts.insert(r, lab); },
                    _ => {}
                }
            }
        }

        Labs { inputs, outputs, boosts }
    }

    pub(crate) fn inputs(&self) -> &[StructureLab] {
        &self.inputs
    }

    pub(crate) fn outputs(&self) -> &[StructureLab] {
        &self.outputs
    }

    fn boosts(&self) -> &HashMap<ResourceType, StructureLab> {
        &self.boosts
    }

    pub(crate) fn boost_lab(&self, resource: &ResourceType) -> Option<ObjectId<StructureLab>> {
        self.boosts.get(resource)
            .map(|lab| lab.id())
    }
}