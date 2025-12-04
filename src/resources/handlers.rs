use std::{collections::HashMap, cmp::max};

use screeps::{ResourceType, game};

use crate::{resources::{MIN_LAB_PRODUCTION, Resources, RoomContext, chain_config::factory_chain_config}, rooms::{RoomEvent, state::requests::{CarryData, FactoryData, LabData, Request, RequestKind, assignment::Assignment}}};

pub type ResourceHandlerFn =
    fn(ResourceType, u32, &Resources, &RoomContext) -> Option<RoomEvent>;

pub struct ResourceHandlers;
impl ResourceHandlers {
    pub fn handle(
        &self,
        res: ResourceType,
        amount: u32,
        room: &Resources,
        ctx: &RoomContext,
    ) -> Option<RoomEvent> {
        let f = room_handler_for(res);
        f(res, amount, room, ctx)
    }
    // pub fn handle(
    //     &self,
    //     res: ResourceType,
    //     room: &Resources,
    //     ctx: &RoomContext,
    // ) -> Option<RoomEvent> {
    //     let f = room_handler_for(res);
    //     f(res, room, ctx)
    // }
}

pub fn room_handler_for(res: ResourceType) -> ResourceHandlerFn {
    use ResourceType::*;

    match res {
        Energy => energy_handler,
        Power => power_handler,
        Ops => ops_handler,

        Oxygen
        | Hydrogen
        | Zynthium
        | Keanium
        | Catalyst
        | Utrium
        | Lemergium => mineral_handler,

        Hydroxide
        | ZynthiumKeanite
        | UtriumLemergite
        | Ghodium
        | UtriumHydride
        | UtriumOxide
        | KeaniumHydride
        | KeaniumOxide
        | LemergiumHydride
        | LemergiumOxide
        | ZynthiumHydride
        | ZynthiumOxide
        | GhodiumHydride
        | GhodiumOxide => reaction_first_tier,
        
        UtriumAcid
        | UtriumAlkalide
        | KeaniumAcid
        | KeaniumAlkalide
        | LemergiumAcid
        | LemergiumAlkalide
        | ZynthiumAcid
        | ZynthiumAlkalide
        | GhodiumAcid
        | GhodiumAlkalide => reaction_second_tier,

        CatalyzedGhodiumAcid
        | CatalyzedGhodiumAlkalide
        | CatalyzedKeaniumAcid
        | CatalyzedKeaniumAlkalide
        | CatalyzedLemergiumAcid
        | CatalyzedLemergiumAlkalide
        | CatalyzedUtriumAcid
        // | CatalyzedUtriumAlkalide //+600% harvest effectiveness, don't need?
        | CatalyzedZynthiumAcid
        | CatalyzedZynthiumAlkalide => reaction_third_tier,

        Metal
        | Alloy
        | Tube
        | Biomass
        | Cell
        | Phlegm
        | Tissue
        | Muscle
        | Silicon
        | Wire
        | Switch
        | Transistor
        | Mist
        | Condensate
        | Concentrate
        | Extract => factory_chain_handler,

        Microchip
        | Organoid
        | Emanation
        | Frame => sellable_handler,

        _ => default_handler,
    }
}

fn sellable_handler(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    (amount > 0)
        .then(|| RoomEvent::Excess(res, amount))
}

fn factory_chain_handler(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    ctx: &RoomContext,
) -> Option<RoomEvent> {
    let cfg = factory_chain_config(res)?;

    (amount > cfg.limit)
        .then(|| {
            if ctx.fl == cfg.chain.f_lvl {
                RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(cfg.chain.resource, cfg.chain.amount)),
                    Assignment::None))
            } else if let Some(other) = cfg.opt1
                .filter(|other| ctx.fl == other.f_lvl)
                .or(cfg.opt2)
                .filter(|other| ctx.fl == other.f_lvl)
            {
                RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(other.resource, other.amount)),
                    Assignment::None))
            } else {
                RoomEvent::Excess(res, cfg.limit)
            }
        })
}

fn reaction_first_tier(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    if amount < 5_000 {
        Some(RoomEvent::Request(Request::new(
            RequestKind::Lab(LabData::new(
                res,
                max(MIN_LAB_PRODUCTION, 5_000 - amount))),
            Assignment::None)))
    }
    // else if GhodiumOxide && amount > 10000 {
    //     //todo lab.reverseReaction(lab1, lab2)
    //     None
    // }
    else {
        None
    }
}

fn reaction_second_tier(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    (amount < 3_000)
        .then(|| RoomEvent::Request(Request::new(
            RequestKind::Lab(LabData::new(
                res,
                max(MIN_LAB_PRODUCTION, 3_000 - amount))),
            Assignment::None)))
}

fn reaction_third_tier(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    // if CatalyzedGhodiumAcid 1_000 Lack ?
    if amount < 3_000 {
        if game::time() % 200 == 0 {
            Some(RoomEvent::Lack(res, 3_000))
        } else {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Lab(LabData::new(
                    res,
                    max(MIN_LAB_PRODUCTION, 3_000 - amount))),
                Assignment::None)))
        }
    } else if amount < 10000 {
        Some(RoomEvent::Request(Request::new(
            RequestKind::Lab(LabData::new(
                ResourceType::CatalyzedUtriumAcid,
                max(MIN_LAB_PRODUCTION, 10_000 - amount))),
            Assignment::None)))
    } else {
        None
    }
}

fn default_handler(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    None
    // if amount < 10_000 {
    //     Some(RoomEvent::Lack(res, 3_000))
    // } else if amount > 50_000 {
    //     Some(RoomEvent::Excess(res, amount - 50_000))
    // } else {
    //     None
    // }
}

fn energy_handler(
    _res: ResourceType,
    energy: u32,
    resources: &Resources,
    ctx: &RoomContext,
) -> Option<RoomEvent> {
    let battery = resources.amount(ResourceType::Battery);

    match ctx.rcl {
        8 if energy > 300_000 => Some(RoomEvent::Request(Request::new(
            RequestKind::Factory(FactoryData::new(ResourceType::Battery, 5000)),
            Assignment::None))),
        8 if battery > 20_000 => Some(RoomEvent::Excess(ResourceType::Battery, battery - 20_000)),
        7 | 8 if energy < 50_000 && battery >= 50 => Some(RoomEvent::Request(Request::new(
            RequestKind::Factory(FactoryData::new(ResourceType::Energy, 50000)),
            Assignment::None))),
        7 | 8 if energy < 50_000 => Some(RoomEvent::Lack(ResourceType::Battery, 5000)),
        6 if energy < 50_000 && ctx.terminal.is_some() => Some(RoomEvent::Lack(ResourceType::Energy, 50000)),
        _ => None
    }
}

fn power_handler(
    _res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    (amount < 10_000)
        .then(|| RoomEvent::Lack(ResourceType::Power, 10_000))
}

fn ops_handler(
    _res: ResourceType,
    ops: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    if ops < 10_000 {
        Some(RoomEvent::Lack(ResourceType::Ops, 3_000))
    } else if ops > 50_000 {
        Some(RoomEvent::Excess(ResourceType::Ops, ops - 50_000))
    } else {
        None
    }
}

fn mineral_handler(
    res: ResourceType,
    amount: u32,
    resources: &Resources,
    ctx: &RoomContext,
) -> Option<RoomEvent> {
    if amount > 50_000 && let Some(compressed_resource) = get_compressed_resource(res) {
        let compressed_amount = resources.amount(compressed_resource);
        if compressed_amount > 100_000 && let (Some(t_id), Some(s_id)) = (ctx.terminal, ctx.storage) {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Carry(
                    CarryData::new(
                        s_id,
                        t_id,
                        res,
                        amount - 50_000)),
                Assignment::Single(None))))
        } else {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Factory(FactoryData::new(compressed_resource, 5_000)),
                Assignment::None)))
        }
    } else {
        None
    }
}

fn get_compressed_resource(resource: ResourceType) -> Option<ResourceType> {
    resource
        .commodity_recipe()
        .and_then(|recipe| recipe.components.iter()
            .find_map(|(component, _)| {
                if *component != ResourceType::Energy {
                    Some(*component)
                } else {
                    None
                }
            }))
}