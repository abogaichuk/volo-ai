use std::collections::HashMap;

use screeps::{RawObjectId, ResourceType, RoomName};

use crate::colony::events::ColonyContext;
use crate::resources::handlers::get_handler_for;
use crate::rooms::RoomEvent;

// mod policy;
pub mod chain_config;
mod handlers;

const MIN_LAB_PRODUCTION: u32 = 5;

//todo statistics and colony_events resource handlers
pub struct RoomContext {
    pub rcl: u8,
    pub terminal: Option<RawObjectId>,
    pub storage: Option<RawObjectId>,
    pub fl: u8,
}

impl RoomContext {
    pub fn new(
        rcl: u8,
        terminal: Option<RawObjectId>,
        storage: Option<RawObjectId>,
        fl: u8,
    ) -> Self {
        Self { rcl, terminal, storage, fl }
    }
}

pub struct Resources {
    amounts: HashMap<ResourceType, u32>,
}

impl Resources {
    pub fn new(amounts: HashMap<ResourceType, u32>) -> Self {
        Self { amounts }
    }

    pub fn amount(&self, res: ResourceType) -> u32 {
        *self.amounts.get(&res).unwrap_or(&0)
    }

    pub fn all(&self) -> &HashMap<ResourceType, u32> {
        &self.amounts
    }

    pub fn events<'a>(&'a self, ctx: RoomContext) -> impl Iterator<Item = RoomEvent> + 'a {
        self.amounts
            .iter()
            .filter_map(move |(res, amount)| get_handler_for(*res)(*res, *amount, self, &ctx))
    }
}

pub struct ResourceOnLowResult {
    amount: u32,
    room_name: RoomName,
}

impl ResourceOnLowResult {
    pub fn amount(&self) -> u32 {
        self.amount
    }

    pub fn room_name(&self) -> RoomName {
        self.room_name
    }
}

pub type ResourceOnLowHandlerFn =
    fn(ResourceType, u32, &ColonyContext) -> Option<ResourceOnLowResult>;

pub fn lack_handler_for(res: ResourceType) -> ResourceOnLowHandlerFn {
    use ResourceType::*;

    match res {
        Energy | Battery | CatalyzedGhodiumAcid => contain_excessive,
        _ => divide_by_half,
    }
}

fn divide_by_half(
    res: ResourceType,
    amount: u32,
    ctx: &ColonyContext,
) -> Option<ResourceOnLowResult> {
    find_room_with_high_amount(res, ctx).filter(|(_, available)| *available > 0).map(
        |(room_name, available)| {
            if available > amount * 2 {
                ResourceOnLowResult { room_name, amount }
            } else if available >= amount {
                ResourceOnLowResult { room_name, amount: amount / 2 }
            } else {
                ResourceOnLowResult { room_name, amount: available / 2 }
            }
        },
    )
}

fn contain_excessive(
    res: ResourceType,
    amount: u32,
    ctx: &ColonyContext,
) -> Option<ResourceOnLowResult> {
    find_room_with_high_amount(res, ctx)
        .filter(|(_, available)| *available > amount * 2)
        .map(|(room_name, _)| ResourceOnLowResult { room_name, amount })
}

fn find_room_with_high_amount(res: ResourceType, ctx: &ColonyContext) -> Option<(RoomName, u32)> {
    ctx.bases()
        .values()
        .map(|b| (b.get_name(), b.resources.amount(res)))
        .max_by_key(|(_, available)| *available)
}
