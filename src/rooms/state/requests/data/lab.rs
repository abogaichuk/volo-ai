use log::*;
use serde::{Serialize, Deserialize};
use screeps::{ResourceType, action_error_codes::RunReactionErrorCode, game};
use smallvec::SmallVec;
use crate::{
    rooms::{RoomEvent, shelter::{Labs, Shelter}, state::requests::{Meta, Status}},
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
    home: &Shelter
) -> SmallVec<[RoomEvent; 3]> {

    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    //split production labs by inputs and otputs
    let Labs { inputs, outputs, boosts: _ } = home.labs();

    if !events.is_empty() || inputs.len() != 2 {
        warn!("{} labs didn't set!", home.name());
        return events;
    }

    match meta.status {
        Status::InProgress => {
            if let Some([(res1, input1), (res2, input2)]) = data.resource.reaction_components()
                .and_then(|components| components.into_iter().zip(inputs).next_chunk().ok())
            {
                let mut aborted = false;
                if let Some(unload_event) = home.unload(input1, &[res1]) {
                    events.push(unload_event);
                } else if let Some(unload_event) = home.unload(input2, &[res2]) {
                    events.push(unload_event);
                } else {
                    for output in outputs.into_iter()
                        .filter(|o| o.cooldown() == 0)
                    {
                        match output.run_reaction(input1, input2) {
                            Ok(_) => {
                                if LAB_PRODUCTION >= data.amount {
                                    meta.update(Status::Finished);
                                } else {
                                    data.amount -= LAB_PRODUCTION
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

                                        if let Some(load_event) = home.load_lab(lab, (res, MIN_CARRY_REQUEST_AMOUNT)) {
                                            meta.update(Status::OnHold);
                                            events.push(load_event);
                                        } else {
                                            let in_lab_amount = lab.store().get_used_capacity(Some(res));
                                            debug!("{} can't find missing component: {} for request: {:?}", home.name(), res, data);
                                            aborted = true;
                                            events.push(RoomEvent::Lack(res, data.amount - in_lab_amount));
                                            break;
                                        }
                                    }
                                    RunReactionErrorCode::Full | RunReactionErrorCode::InvalidArgs => {
                                        events.extend(home.unload(output, &[]));
                                    }
                                    _ => {
                                        aborted = true;
                                        error!("lab error: {:?}", err);
                                    }
                                }
                            }
                        }
                    };
                }
                if aborted {
                    meta.update(Status::Aborted);
                }
            } else {
                meta.update(Status::Aborted);
                warn!("{} can't get reagents for lab request: {:?}", home.name(), data);
            }
        }
        Status::OnHold => {
            if meta.updated_at + 10 < game::time() {
                meta.update(Status::InProgress);
            }
        }
        //wait 20 tick to prevent duplication request creatation
        Status::Finished if meta.updated_at + 20 < game::time() => {
            meta.update(Status::Resolved);
        }
        _ => {}
    }

    events
}