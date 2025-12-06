use log::*;
use serde::{Serialize, Deserialize};
use screeps::{FactoryRecipe, HasId, ResourceType, StructureFactory, action_error_codes::ProduceErrorCode, game};
use smallvec::SmallVec;
use std::cmp::min;
use crate::{
    rooms::{RoomEvent, shelter::Shelter, state::requests::{Meta, Status}},
    utils::constants::MAX_CARRY_REQUEST_AMOUNT
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FactoryData {
    pub resource: ResourceType,
    #[serde(default)]
    pub amount: u32,
}

impl FactoryData {
    pub fn new(resource: ResourceType, amount: u32) -> Self {
        Self { resource, amount }
    }
}

pub(in crate::rooms::state::requests) fn factory_handler(
    data: &mut FactoryData,
    meta: &mut Meta,
    home: &Shelter
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let Some(factory) = home.factory() else {
        return events;
    };

    let recipe = data.resource.commodity_recipe().expect("expect commodity receipe");
    match meta.status {
        Status::InProgress => {
            //if free space
            if factory.store().get_free_capacity(None) >= MAX_CARRY_REQUEST_AMOUNT as i32 {
                if let Some(component) = get_missing_component(factory, &recipe) {
                    let request_amount = component.1 * (data.amount / recipe.amount);
                    let factory_amount = factory.store().get_used_capacity(Some(component.0));

                    if let Some(load_event) = home.supply_resources(
                        factory.raw_id(),
                        component.0,
                        min(request_amount - factory_amount, MAX_CARRY_REQUEST_AMOUNT))
                    {
                        events.push(load_event);
                        meta.update(Status::OnHold);
                    } else {
                        debug!("{} can't find missing component: {} for request: {:?}", home.name(), component.0, data);
                        meta.update(Status::Aborted);
                    }
                }
                // else if recipe.level.is_some() && !home.is_power_enabled(&screeps::PowerType::OperateFactory) {
                //     events.push(RoomEvent::AddPower(screeps::PowerType::OperateFactory));
                // }
                else if recipe.level.is_none() && home.is_power_enabled(&screeps::PowerType::OperateFactory) {
                    events.push(RoomEvent::DeletePower(screeps::PowerType::OperateFactory));
                } else if factory.cooldown() == 0 {
                    match factory.produce(data.resource) {
                        Ok(_) => {
                            debug!("{} factory produced: OK request.amount: {}", home.name(), data.amount);
                            if recipe.amount >= data.amount {
                                info!("{} factory finished request: {}", home.name(), data.resource);
                                meta.update(Status::Resolved);
                            } else {
                                data.amount -= recipe.amount;
                                debug!("{} new request.amount: {}", home.name(), data.amount);
                            }
                        }
                        Err(err) => {
                            match err {
                                //if busy wait for powercreep effect
                                ProduceErrorCode::Busy => {
                                    events.push(RoomEvent::AddPower(screeps::PowerType::OperateFactory));
                                },
                                _ => {
                                    meta.update(Status::Aborted);
                                    error!("{} factory error: {:?}, request: {:?}", home.name(), err, data)
                                }
                            }
                        }
                    }
                }
            } else if let Some(unload_event) = home.unload(factory, &recipe.components.keys().cloned().collect::<Vec<_>>()) {
                events.push(unload_event);
            } else {
                meta.update(Status::Aborted);
                warn!("{} no factory free amount and nothing to unload: close: {:?}", home.name(), data);
            }
        }
        Status::OnHold => {
            if recipe.components.iter()
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
    };
    events
}

fn get_missing_component(factory: &StructureFactory, recipe: &FactoryRecipe) -> Option<(ResourceType, u32)> {
    recipe.components.clone()
        .into_iter()
        .find(|(resource, amount)| factory.store().get_used_capacity(Some(*resource)) < *amount)
}