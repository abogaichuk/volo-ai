use std::cmp;

use log::{debug, error, warn};
use rand::seq::SliceRandom;
use rand::thread_rng;
use screeps::{
    HasId, HasPosition, ResourceType, StructureLab,
    action_error_codes::{ReverseReactionErrorCode, RunReactionErrorCode},
    game,
};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::rooms::{
    RoomEvent,
    shelter::Shelter,
    state::requests::{
        CarryData, Meta, Request, RequestKind, Status, WithdrawData, assignment::Assignment,
    },
};
use crate::utils::constants::MIN_CARRY_REQUEST_AMOUNT;

const LAB_PRODUCTION: u32 = 5;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LabData {
    pub resource: ResourceType,
    pub amount: u32,
    #[serde(default = "default_false")]
    pub reverse: bool,
}

fn default_false() -> bool {
    false
}

impl LabData {
    pub const fn new(resource: ResourceType, amount: u32, reverse: bool) -> Self {
        Self { resource, amount, reverse }
    }
}

#[allow(clippy::similar_names)]
pub(in crate::rooms::state::requests) fn lab_handler(
    data: &mut LabData,
    meta: &mut Meta,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let (inputs, outputs) = home.production_labs();

    if inputs.len() != 2 || meta.updated_at + 2_500 < game::time() || data.amount == 0 {
        debug!("{} labs didn't set or timeout exceed!", home.name());
        meta.update(Status::Aborted);
        return smallvec![];
    }

    if data.reverse {
        reverse_reaction(data, meta, home, outputs, inputs)
    } else {
        reaction(data, meta, home, inputs, outputs)
    }
}

fn reaction(
    data: &mut LabData,
    meta: &mut Meta,
    home: &Shelter,
    inputs: &[StructureLab],
    outputs: &[StructureLab],
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::InProgress => {
            if let Some([(res1, input1), (res2, input2)]) = data
                .resource
                .reaction_components()
                .and_then(|components| components.into_iter().zip(inputs).next_chunk().ok())
            {
                if let Some(unload_event) = home.unload(input1, &[res1]) {
                    events.push(unload_event);
                } else if let Some(unload_event) = home.unload(input2, &[res2]) {
                    events.push(unload_event);
                } else {
                    for output in outputs.iter().filter(|o| o.cooldown() == 0) {
                        match output.run_reaction(input1, input2) {
                            Ok(()) => {
                                if LAB_PRODUCTION > data.amount {
                                    meta.update(Status::Resolved);
                                    break;
                                }
                                data.amount -= LAB_PRODUCTION;
                                meta.update(Status::InProgress);
                            }
                            Err(err) => {
                                match err {
                                    RunReactionErrorCode::NotEnoughResources => {
                                        let event1 = if input1.store().get_used_capacity(Some(res1))
                                            < LAB_PRODUCTION
                                        {
                                            if let Some(event) = home.load_lab(
                                                input1,
                                                (
                                                    res1,
                                                    cmp::min(data.amount, MIN_CARRY_REQUEST_AMOUNT),
                                                ),
                                            ) {
                                                Some(event)
                                            } else {
                                                warn!(
                                                    "{} not found resource: {}",
                                                    home.name(),
                                                    res1
                                                );
                                                meta.update(Status::Aborted);
                                                break;
                                            }
                                        } else {
                                            None
                                        };

                                        let event2 = if input2.store().get_used_capacity(Some(res2))
                                            < LAB_PRODUCTION
                                        {
                                            if let Some(event) = home.load_lab(
                                                input2,
                                                (
                                                    res2,
                                                    cmp::min(data.amount, MIN_CARRY_REQUEST_AMOUNT),
                                                ),
                                            ) {
                                                Some(event)
                                            } else {
                                                warn!(
                                                    "{} not found resource: {}",
                                                    home.name(),
                                                    res2
                                                );
                                                meta.update(Status::Aborted);
                                                break;
                                            }
                                        } else {
                                            None
                                        };

                                        events.extend(event1);
                                        events.extend(event2);
                                        meta.update(Status::OnHold);
                                    }
                                    RunReactionErrorCode::Full
                                    | RunReactionErrorCode::InvalidArgs => {
                                        events.extend(home.unload(output, &[]));
                                    }
                                    _ => {
                                        error!("lab error: {:?}", err);
                                        meta.update(Status::Aborted);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            } else {
                meta.update(Status::Aborted);
                warn!("{} can't get reagents for lab request: {:?}", home.name(), data);
            }
        }
        Status::OnHold => {
            if inputs.iter().all(|lab| {
                data.resource.reaction_components().is_some_and(|components| {
                    components.iter().any(|component| {
                        lab.store().get_used_capacity(Some(*component)) >= LAB_PRODUCTION
                    })
                })
            }) {
                // if all labs loaded -> toogle to InProgress
                meta.update(Status::InProgress);
            } else if meta.updated_at + 50 < game::time() {
                // if wait more then 50 ticks  -> Aborted
                meta.update(Status::Aborted);
            }
        }
        _ => {}
    };
    events
}

fn reverse_reaction(
    data: &mut LabData,
    meta: &mut Meta,
    home: &Shelter,
    inputs: &[StructureLab],
    outputs: &[StructureLab], //2 lab
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::InProgress => {
            if let Some([(res1, output1), (res2, output2)]) = data
                .resource
                .reaction_components()
                .and_then(|components| components.into_iter().zip(outputs).next_chunk().ok())
            {
                if let Some(unload_event) = home.unload(output1, &[res1]) {
                    events.push(unload_event);
                } else if let Some(unload_event) = home.unload(output2, &[res2]) {
                    events.push(unload_event);
                } else {
                    let mut waiting_for_load: u32 = 0;
                    for input in inputs {
                        if input.store().get_used_capacity(Some(data.resource)) >= LAB_PRODUCTION {
                            match input.reverse_reaction(output1, output2) {
                                Ok(()) => {
                                    if LAB_PRODUCTION > data.amount {
                                        meta.update(Status::Resolved);
                                        break;
                                    }
                                    data.amount -= LAB_PRODUCTION;
                                    meta.update(Status::InProgress);
                                }
                                Err(err) => match err {
                                    ReverseReactionErrorCode::Full | ReverseReactionErrorCode::InvalidArgs => {
                                        if output1.store().get_used_capacity(None) > 0 {
                                            let event = RoomEvent::Request(Request::new(
                                                RequestKind::Withdraw(WithdrawData::new(
                                                    output1.id().into(),
                                                    output1.pos(),
                                                    output1
                                                        .store()
                                                        .store_types()
                                                        .into_iter()
                                                        .map(|res| (res, None))
                                                        .collect(),
                                                )),
                                                Assignment::Single(None),
                                            ));
                                            events.push(event);
                                        }

                                        if output2.store().get_used_capacity(None) > 0 {
                                            let event = RoomEvent::Request(Request::new(
                                                RequestKind::Withdraw(WithdrawData::new(
                                                    output2.id().into(),
                                                    output2.pos(),
                                                    output2
                                                        .store()
                                                        .store_types()
                                                        .into_iter()
                                                        .map(|res| (res, None))
                                                        .collect(),
                                                )),
                                                Assignment::Single(None),
                                            ));
                                            events.push(event);
                                        }

                                        meta.update(Status::OnHold);
                                    }
                                    ReverseReactionErrorCode::Tired => {} // do nothing
                                    _ => {
                                        error!("{} lab error: {:?}", home.name(), err);
                                        meta.update(Status::Aborted);
                                    }
                                },
                            }
                        } else if let Some(event) = home.unload(input, &[data.resource]) {
                            events.push(event);
                        } else {
                            waiting_for_load += 1;
                        }
                    }

                    if inputs.len() as u32 == waiting_for_load {
                        //all iutputs are empty, split request amount
                        if let Some(storage) = home
                            .storage()
                            .filter(|s| s.store().get_used_capacity(Some(data.resource)) >= data.amount)
                        {
                            if data.amount <= LAB_PRODUCTION * waiting_for_load
                                && let Some(lab) = inputs.get(0)
                            {
                                // if small amount -> load only 1 lab
                                let event = RoomEvent::Request(Request::new(
                                    RequestKind::Carry(CarryData::new(
                                        storage.raw_id(),
                                        lab.raw_id(),
                                        data.resource,
                                        data.amount,
                                    )),
                                    Assignment::Single(None),
                                ));
                                events.push(event);
                            } else {
                                // take 3 labs randomly, because of the contract SmallVec<[RoomEvent; 3] restriction
                                let part = data.amount / waiting_for_load;
                                let mut rng = thread_rng();
                                events.extend(inputs.choose_multiple(&mut rng, 3).map(|lab| {
                                    RoomEvent::Request(Request::new(
                                        RequestKind::Carry(CarryData::new(
                                            storage.raw_id(),
                                            lab.raw_id(),
                                            data.resource,
                                            part - (part % LAB_PRODUCTION), //should be multiple of 5
                                        )),
                                        Assignment::Single(None),
                                    ))
                                }));
                            }
                        } else {
                            warn!(
                                "lab reverse reaction, not enough res {}:{}",
                                data.resource, data.amount
                            );
                            meta.update(Status::Aborted);
                        }
                    }
                }
            } else {
                meta.update(Status::Aborted);
                warn!("{} can't get reagents for lab request: {:?}", home.name(), data);
            }
        }
        Status::OnHold => {
            if data
                .resource
                .reaction_components()
                .map(|components| {
                    components.into_iter().zip(outputs).all(|(res, lab)| {
                        lab.store().get_free_capacity(Some(res)) >= LAB_PRODUCTION as i32
                    })
                })
                .unwrap_or(false)
            {
                // if all labs unloaded -> toogle to InProgress
                meta.update(Status::InProgress);
            } else if meta.updated_at + 50 < game::time() {
                // if wait more then 50 ticks  -> Aborted
                meta.update(Status::Aborted);
            }
        }
        _ => {}
    };
    events
}
