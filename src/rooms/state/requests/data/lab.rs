use std::cmp;

use log::*;
use serde::{Serialize, Deserialize};
use screeps::{ResourceType, action_error_codes::RunReactionErrorCode, game};
use smallvec::SmallVec;
use crate::{
    rooms::{RoomEvent, state::requests::{Meta, Status}, wrappers::claimed::Claimed},
    utils::constants::MIN_CARRY_REQUEST_AMOUNT
};

const LAB_PRODUCTION: u32 = 5;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LabData {
    pub resource: ResourceType,
    pub amount: u32,
}

impl LabData {
    pub fn new(resource: ResourceType, amount: u32) -> Self {
        Self { resource, amount }
    }
}

pub(in crate::rooms::state::requests) fn lab_handler(
    data: &mut LabData,
    meta: &mut Meta,
    home: &Claimed
) -> SmallVec<[RoomEvent; 3]> {

    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let (inputs, outputs)= home.production_labs();

    if inputs.len() != 2 {
        warn!("{} labs didn't set!", home.get_name());
        return events;
    }

    match meta.status {
        Status::InProgress => {
            if let Some([(res1, input1), (res2, input2)]) = data.resource.reaction_components()
                .and_then(|components| components.into_iter().zip(inputs).next_chunk().ok())
            {
                if let Some(unload_event) = home.unload(input1, &[res1]) {
                    events.push(unload_event);
                } else if let Some(unload_event) = home.unload(input2, &[res2]) {
                    events.push(unload_event);
                } else {
                    for output in outputs.iter()
                        .filter(|o| o.cooldown() == 0)
                    {
                        match output.run_reaction(input1, input2) {
                            Ok(_) => {
                                if LAB_PRODUCTION > data.amount {
                                    meta.update(Status::Finishing);
                                    break;
                                } else {
                                    data.amount -= LAB_PRODUCTION;
                                    meta.update(Status::InProgress);
                                }
                            }
                            Err(err) => {
                                match err {
                                    RunReactionErrorCode::NotEnoughResources => {
                                        let (res, lab) = if input1.store().get_used_capacity(Some(res1)) < LAB_PRODUCTION {
                                            (res1, input1)
                                        } else {
                                            (res2, input2)
                                        };

                                        let in_lab_amount = lab.store().get_used_capacity(Some(res));
                                        let amount = cmp::min(data.amount, MIN_CARRY_REQUEST_AMOUNT)
                                            .saturating_sub(in_lab_amount);

                                        if let Some(load_event) = home.load_lab(lab, (res, amount)) {
                                            meta.update(Status::InProgress);
                                            events.push(load_event);
                                        } else if meta.updated_at + 15 < game::time() {
                                            //15 ticks to deliver resources to the lab
                                            debug!("{} can't find missing component: {} for request: {:?}", home.get_name(), res, data);
                                            meta.update(Status::Aborted);
                                            events.push(RoomEvent::Lack(res, data.amount - in_lab_amount));
                                        }
                                    }
                                    RunReactionErrorCode::Full | RunReactionErrorCode::InvalidArgs => {
                                        events.extend(home.unload(output, &[]));
                                    }
                                    _ => {
                                        error!("lab error: {:?}", err);
                                        meta.update(Status::Aborted);
                                    }
                                };
                                break;
                            }
                        }
                    };
                }
            } else {
                meta.update(Status::Aborted);
                warn!("{} can't get reagents for lab request: {:?}", home.get_name(), data);
            }
        }
        //wait 50 tick to prevent duplication request creatation
        //todo instead of this create the Resource struct for each claimed room
        //so each home has an info about resource amount.
        //what about resources in a creeps store?
        Status::Finishing if meta.updated_at + 50 < game::time() => {
            meta.update(Status::Resolved);
        }
        _ => {}
    }

    events
}