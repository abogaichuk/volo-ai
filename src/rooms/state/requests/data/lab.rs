use std::cmp;

use log::{debug, error, warn};
use screeps::{ResourceType, action_error_codes::RunReactionErrorCode, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::{
    RoomEvent,
    shelter::Shelter,
    state::requests::{Meta, Status},
};
use crate::utils::constants::MIN_CARRY_REQUEST_AMOUNT;

const LAB_PRODUCTION: u32 = 5;

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
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let (inputs, outputs) = home.production_labs();

    if inputs.len() != 2 || meta.updated_at + 2_500 < game::time() {
        debug!("{} labs didn't set or timeout exceed!", home.name());
        meta.update(Status::Aborted);
        return events;
    }

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
    }

    events
}
