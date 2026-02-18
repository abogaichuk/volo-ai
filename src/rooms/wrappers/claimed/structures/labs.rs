use std::{cmp, collections::HashMap};

use itertools::Itertools;
use log::debug;
use screeps::{HasId, HasPosition, ObjectId, ResourceType, StructureLab};

use crate::commons::find_container_with;
use crate::rooms::{
    RoomEvent,
    shelter::Shelter,
    state::{
        constructions::{LabStatus, PlannedCell, RoomPlan, RoomStructure},
        requests::{CarryData, Request, RequestKind, assignment::Assignment, meta::Status},
    },
};
use crate::utils::constants::LAB_PRODUCTION;

const MIN_ENERGY_AMOUNT: u32 = 1000;
const MIN_RESOURCE_AMOUNT: u32 = 2000;

impl Shelter<'_> {
    pub(crate) fn run_labs(&self) -> Option<RoomEvent> {
        debug!("{} running labs", self.name());

        //update lab statuses to current boosts
        self.update_lab_state()
            .or_else(|| {
                self.base
                    .labs
                    .boosts()
                    .iter()
                    //load resources for boost
                    .find_map(|(res, lab)| self.keep_boost_ready(*res, lab))
            })
            .or_else(|| {
                (!self.are_labs_busy())
                    .then(|| {
                        if let Some(mut request) = self.get_lab_request() {
                            request.join(None, None);
                            Some(RoomEvent::ReplaceRequest(request))
                        } else {
                            //no requests found, clear the labs
                            self.base
                                .labs
                                .inputs()
                                .iter()
                                .chain(self.base.labs.outputs.iter())
                                .find_map(|lab| self.unload(lab, &[]))
                        }
                    })
                    .flatten()
            })
    }

    fn update_lab_state(&self) -> Option<RoomEvent> {
        //all unique boostable resources
        let boost_resources: Vec<ResourceType> = self
            .state
            .boosts
            .iter()
            .flat_map(|boost_reason| boost_reason.0.value())
            .unique()
            .collect();

        boost_resources
            .iter()
            .find_map(|res| {
                (!self.base.labs.boosts().contains_key(res)).then(|| {
                    self.base.labs.outputs.first().map(|lab| {
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
                self.base.labs.boosts.iter().find_map(|(res, lab)| {
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

    fn keep_boost_ready(&self, resource: ResourceType, lab: &StructureLab) -> Option<RoomEvent> {
        self.load_lab(lab, (ResourceType::Energy, MIN_ENERGY_AMOUNT)) //supply energy
            .or_else(|| {
                self.unload(lab, &[resource]) //unload resources
                    //load boost resource
                    .or_else(|| self.load_lab(lab, (resource, MIN_RESOURCE_AMOUNT)))
            })
    }

    //todo might be a bug -> replaces existed request by a new one
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

    fn are_labs_busy(&self) -> bool {
        self.state.requests.iter().any(|r| {
            matches!(r.kind, RequestKind::Lab(_))
                && matches!(r.status(), Status::InProgress | Status::OnHold)
        })
    }

    fn get_lab_request(&self) -> Option<Request> {
        let storage = self.base.storage()?;

        self.requests()
            .find(|r| match &r.kind {
                RequestKind::Lab(d) => {
                    if d.reverse && storage.store().get_used_capacity(Some(d.resource)) >= d.amount
                    {
                        true
                    } else {
                        d.resource.reaction_components().is_some_and(|components| {
                            components.iter().all(|component| {
                                storage.store().get_used_capacity(Some(*component))
                                    >= LAB_PRODUCTION
                            })
                        })
                    }
                }
                _ => false,
            })
            .cloned()
    }
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

    pub(crate) fn boost_lab(&self, resource: ResourceType) -> Option<ObjectId<StructureLab>> {
        self.boosts.get(&resource).map(screeps::HasId::id)
    }
}
