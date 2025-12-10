use std::cmp;

use log::{error, warn};
use screeps::action_error_codes::RunReactionErrorCode;
use screeps::{ResourceType, StructureLab, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use thiserror::Error;

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Meta, Status};
use crate::rooms::wrappers::claimed::Claimed;
use crate::utils::constants::MIN_CARRY_REQUEST_AMOUNT;

const LAB_PRODUCTION: u32 = 5;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum LabError {
    #[error("not found: {0}")]
    NotFound(ResourceType),
    #[error("is not empty")]
    IsNotEmpty,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LabData {
    pub resource: ResourceType,
    pub amount: u32,
}

impl LabData {
    pub const fn new(resource: ResourceType, amount: u32) -> Self {
        Self { resource, amount }
    }
}

#[allow(clippy::similar_names)]
pub(in crate::rooms::state::requests) fn lab_handler(
    data: &mut LabData,
    meta: &mut Meta,
    home: &Claimed,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let (inputs, outputs) = home.production_labs();

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

                                        let event1 = match try_supply(
                                            home,
                                            input1,
                                            res1,
                                            cmp::min(data.amount, MIN_CARRY_REQUEST_AMOUNT))
                                        {
                                            Ok(event) => Some(event),
                                            Err(err) => match err {
                                                LabError::IsNotEmpty => None,
                                                LabError::NotFound(res) => {
                                                    warn!("{} not found resource: {}", home.get_name(), res);
                                                    meta.update(Status::Aborted);
                                                    break;
                                                }
                                            }
                                        };
                                        let event2 = match try_supply(
                                            home,
                                            input2,
                                            res2,
                                            cmp::min(data.amount, MIN_CARRY_REQUEST_AMOUNT))
                                        {
                                            Ok(event) => Some(event),
                                            Err(err) => match err {
                                                LabError::IsNotEmpty => None,
                                                LabError::NotFound(res) => {
                                                    warn!("{} not found resource: {}", home.get_name(), res);
                                                    meta.update(Status::Aborted);
                                                    break;
                                                }
                                            }
                                        };

                                        events.extend(event1);
                                        events.extend(event2);
                                        meta.update(Status::OnHold);
                                    }
                                    RunReactionErrorCode::Full | RunReactionErrorCode::InvalidArgs => {
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
                    };
                }
            } else {
                meta.update(Status::Aborted);
                warn!("{} can't get reagents for lab request: {:?}", home.get_name(), data);
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
    }

    events
}

fn try_supply(
    home: &Claimed,
    lab: &StructureLab,
    res: ResourceType,
    amount: u32,
) -> Result<RoomEvent, LabError> {
    if lab.store().get_used_capacity(Some(res)) < LAB_PRODUCTION {
        home.load_lab(lab, (res, amount)).ok_or(LabError::NotFound(res))
    } else {
        Err(LabError::IsNotEmpty)
    }
}
