use std::cmp;
use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use log::debug;
use screeps::{HasId, HasPosition, ObjectId, ResourceType, StructureLab, StructureStorage};

use crate::commons::find_container_with;
use crate::rooms::RoomEvent;
use crate::rooms::state::constructions::{LabStatus, PlannedCell, RoomPlan, RoomStructure};
use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::meta::Status;
use crate::rooms::state::requests::{CarryData, Request, RequestKind};
use crate::rooms::state::{BoostReason, RoomState};
use crate::rooms::wrappers::claimed::Claimed;

const MIN_ENERGY_AMOUNT: u32 = 1000;
const MIN_RESOURCE_AMOUNT: u32 = 2000;
const LAB_PRODUCTION: u32 = 5;

impl Claimed {
    pub(crate) fn run_labs(&self, state: &RoomState) -> Option<RoomEvent> {
        debug!("{} running labs", self.get_name());

        self.storage().and_then(|storage|
                //update lab statuses to current boosts
                self.update_lab_state(&state.boosts)
                    .or_else(|| self.labs.boosts().iter()
                        //load resources for boost
                        .find_map(|(res, lab)| self.keep_boost_ready(res, lab)))
                    .or_else(|| {
                        let in_progress = state.requests.iter()
                            .any(|r| matches!(r.kind, RequestKind::Lab(_)) &&
                                matches!(r.status(), Status::InProgress | Status::OnHold));

                        // if no in progress request
                        if in_progress {
                            None
                        } else {
                            //take a new one
                            if let Some(mut request) = new_request(&state.requests, storage) {
                                request.join(None, None);
                                Some(RoomEvent::ReplaceRequest(request))
                            } else {
                                //no requests found, clear the labs
                                self.labs.inputs().iter()
                                    .chain(self.labs.outputs.iter())
                                    .find_map(|lab| self.unload(lab, &[]))
                            }
                        }
                    }))
    }

    fn update_lab_state(&self, boosts: &HashMap<BoostReason, u32>) -> Option<RoomEvent> {
        //all unique boostable resources
        let boost_resources: Vec<ResourceType> =
            boosts.iter().flat_map(|boost_reason| boost_reason.0.value()).unique().collect();

        boost_resources
            .iter()
            .find_map(|res| {
                (!self.labs.boosts().contains_key(res)).then(|| {
                    self.labs.outputs.first().map(|lab| {
                        let cell = PlannedCell::searchable(
                            lab.pos().xy(),
                            RoomStructure::Lab(LabStatus::Boost(*res)),
                        );
                        RoomEvent::ReplaceCell(cell)
                    })
                })
            })
            .flatten()
            .or_else(|| {
                self.labs.boosts.iter().find_map(|(res, lab)| {
                    (!boost_resources.contains(res)).then(|| {
                        let cell = PlannedCell::searchable(
                            lab.pos().xy(),
                            RoomStructure::Lab(LabStatus::Output),
                        );
                        RoomEvent::ReplaceCell(cell)
                    })
                })
            })
    }

    fn keep_boost_ready(&self, resource: &ResourceType, lab: &StructureLab) -> Option<RoomEvent> {
        self.load_lab(lab, (ResourceType::Energy, MIN_ENERGY_AMOUNT)) //supply energy
            .or_else(|| {
                self.unload(lab, &[*resource]) //unload resources
                .or_else(|| self.load_lab(lab, (*resource, MIN_RESOURCE_AMOUNT)))
            }) //load boost resource
    }

    pub fn load_lab(
        &self,
        lab: &StructureLab,
        component: (ResourceType, u32),
    ) -> Option<RoomEvent> {
        let in_lab_amount = lab.store().get_used_capacity(Some(component.0));
        (in_lab_amount < component.1)
            .then(|| {
                find_container_with(
                    component.0,
                    None,
                    self.storage(),
                    self.terminal(),
                    self.factory(),
                )
                .map(|(id, amount)| {
                    RoomEvent::Request(Request::new(
                        RequestKind::Carry(CarryData::new(
                            id,
                            lab.raw_id(),
                            component.0,
                            cmp::min(amount, component.1.saturating_sub(in_lab_amount)),
                        )),
                        Assignment::Single(None),
                    ))
                })
            })
            .flatten()
    }
}

fn new_request(requests: &HashSet<Request>, storage: &StructureStorage) -> Option<Request> {
    requests
        .iter()
        .find(|r| match &r.kind {
            RequestKind::Lab(d) => d.resource.reaction_components().is_some_and(|components| {
                components.iter().all(|component| {
                    storage.store().get_used_capacity(Some(*component)) >= LAB_PRODUCTION
                })
            }),
            _ => false,
        })
        .cloned()
}

#[derive(Default)]
pub(crate) struct Labs {
    inputs: Vec<StructureLab>,
    outputs: Vec<StructureLab>,
    boosts: HashMap<ResourceType, StructureLab>,
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
        for lab in labs {
            let cell =
                PlannedCell::searchable(lab.pos().xy(), RoomStructure::Lab(LabStatus::Output));
            if let Some(planned_cell) = plan.get_cell(cell) {
                match planned_cell.structure {
                    RoomStructure::Lab(LabStatus::Input) => inputs.push(lab),
                    RoomStructure::Lab(LabStatus::Output) => outputs.push(lab),
                    RoomStructure::Lab(LabStatus::Boost(r)) => {
                        boosts.insert(r, lab);
                    }
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

    const fn boosts(&self) -> &HashMap<ResourceType, StructureLab> {
        &self.boosts
    }

    pub(crate) fn boost_lab(&self, resource: &ResourceType) -> Option<ObjectId<StructureLab>> {
        self.boosts.get(resource).map(screeps::HasId::id)
    }
}
