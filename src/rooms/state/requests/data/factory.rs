use std::cmp::min;

use log::{debug, error, info, warn};
use screeps::action_error_codes::ProduceErrorCode;
use screeps::{FactoryRecipe, HasId, ResourceType, StructureFactory, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::{CarryData, Meta, Request, RequestKind, Status};
use crate::utils::constants::{MAX_CARRY_REQUEST_AMOUNT, MIN_CARRY_REQUEST_AMOUNT};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FactoryData {
    pub resource: ResourceType,
    #[serde(default)]
    pub amount: u32,
}

impl FactoryData {
    pub const fn new(resource: ResourceType, amount: u32) -> Self {
        Self { resource, amount }
    }
}

pub(in crate::rooms::state::requests) fn factory_handler(
    data: &mut FactoryData,
    meta: &mut Meta,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let Some(factory) = home.factory() else {
        return events;
    };

    let recipe = data.resource.commodity_recipe().expect("expect commodity receipe");
    match meta.status {
        Status::InProgress => {
            //if free space
            if i32::try_from(MAX_CARRY_REQUEST_AMOUNT)
                .ok()
                .is_some_and(|max| factory.store().get_free_capacity(None) >= max)
            {
                if recipe.level.is_none()
                    && home.is_power_enabled(screeps::PowerType::OperateFactory)
                {
                    events.push(RoomEvent::DeletePower(screeps::PowerType::OperateFactory));
                } else if factory.cooldown() == 0 {
                    match factory.produce(data.resource) {
                        Ok(()) => {
                            debug!(
                                "{} factory produced: OK request.amount: {}",
                                home.name(),
                                data.amount
                            );
                            if recipe.amount >= data.amount {
                                info!(
                                    "{} factory finished request: {}",
                                    home.name(),
                                    data.resource
                                );
                                meta.update(Status::Resolved);
                            } else {
                                data.amount -= recipe.amount;
                                debug!("{} new request.amount: {}", home.name(), data.amount);
                            }
                        }
                        Err(err) => {
                            if err == ProduceErrorCode::Busy {
                                events
                                    .push(RoomEvent::AddPower(screeps::PowerType::OperateFactory));
                            } else if err == ProduceErrorCode::NotEnoughResources {
                                let mut load_events = Vec::new();
                                for (comp_res, comp_amount) in
                                    get_missing_components(factory, &recipe)
                                {
                                    let request_amount =
                                        comp_amount * (data.amount / recipe.amount);
                                    let factory_amount =
                                        factory.store().get_used_capacity(Some(comp_res));

                                    if let Some(load_event) = home.storage().and_then(|storage| {
                                        let storage_capacity =
                                            storage.store().get_used_capacity(Some(comp_res));
                                        if storage_capacity >= comp_amount {
                                            Some(RoomEvent::Request(Request::new(
                                                RequestKind::Carry(CarryData::new(
                                                    storage.raw_id(),
                                                    factory.raw_id(),
                                                    comp_res,
                                                    min(
                                                        request_amount - factory_amount,
                                                        min(
                                                            storage_capacity,
                                                            MIN_CARRY_REQUEST_AMOUNT,
                                                        ),
                                                    ),
                                                )),
                                                Assignment::Single(None),
                                            )))
                                        } else {
                                            None
                                        }
                                    }) {
                                        load_events.push(load_event);
                                    } else {
                                        meta.update(Status::Aborted);
                                        break;
                                    }
                                }

                                events.extend(load_events);
                                meta.update(Status::OnHold);
                            } else {
                                meta.update(Status::Aborted);
                                error!(
                                    "{} factory error: {:?}, request: {:?}",
                                    home.name(),
                                    err,
                                    data
                                );
                            }
                        }
                    }
                }
            } else if let Some(unload_event) =
                home.unload(factory, &recipe.components.keys().copied().collect::<Vec<_>>())
            {
                events.push(unload_event);
            } else {
                meta.update(Status::Aborted);
                warn!(
                    "{} no factory free amount and nothing to unload: close: {:?}",
                    home.name(),
                    data
                );
            }
        }
        Status::OnHold => {
            if recipe
                .components
                .iter()
                .all(|(res, amount)| factory.store().get_used_capacity(Some(*res)) >= *amount)
            {
                meta.update(Status::InProgress);
            } else if meta.updated_at + 25 < game::time() {
                meta.update(Status::Aborted);
            }
        }
        //todo wait 20 tick to prevent duplication request creatation
        // Status::Resolved if meta.updated_at + 20 > game::time() => {
        //     events.push(RoomEvent::Request(RoomRequest::Factory(self)));
        // }
        _ => {}
    }
    events
}

fn get_missing_components(
    factory: &StructureFactory,
    recipe: &FactoryRecipe,
) -> impl Iterator<Item = (ResourceType, u32)> {
    recipe
        .components
        .clone()
        .into_iter()
        .filter(|(resource, amount)| factory.store().get_used_capacity(Some(*resource)) < *amount)
}
