use std::cmp::max;

use screeps::{ResourceType, game};

use crate::resources::chain_config::factory_chain_config;
use crate::resources::{MIN_LAB_PRODUCTION, Resources, RoomContext};
use crate::rooms::RoomEvent;
use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::{CarryData, FactoryData, LabData, Request, RequestKind};

pub type ResourceRoomHandlerFn =
    fn(ResourceType, u32, &Resources, &RoomContext) -> Option<RoomEvent>;

pub fn get_handler_for(res: ResourceType) -> ResourceRoomHandlerFn {
    use ResourceType::*;

    match res {
        Energy => energy_handler,
        Battery => battery_handler,
        Power => power_handler,
        Ops => ops_handler,
        GhodiumMelt => ghodium_melt_handler,
        Composite => composite_handler,

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

        Purifier
        | UtriumBar
        | LemergiumBar
        | KeaniumBar
        | ZynthiumBar
        | Reductant
        | Oxidant => compressed_commodities_handler,

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
    (amount > 0).then(|| RoomEvent::Excess(res, amount))
}

fn factory_chain_handler(
    res: ResourceType,
    amount: u32,
    resources: &Resources,
    ctx: &RoomContext,
) -> Option<RoomEvent> {
    let cfg = factory_chain_config(res)?;

    (amount >= cfg.limit).then(|| {
        if let Some(chain) = cfg
            .opt2
            .as_ref()
            .filter(|c| c.f_lvl == ctx.fl)
            .or_else(|| cfg.opt1.as_ref().filter(|c| c.f_lvl == ctx.fl))
            .or(if cfg.chain.f_lvl == ctx.fl { Some(&cfg.chain) } else { None })
        {
            if let Some(missed) = get_missed_component(chain.resource, resources) {
                RoomEvent::Lack(missed.0, missed.1 * 10)
            } else {
                RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(chain.resource, chain.amount)),
                    Assignment::None,
                ))
            }
        } else {
            RoomEvent::Excess(res, cfg.limit)
        }
    })
}

fn get_missed_component(res: ResourceType, all: &Resources) -> Option<(ResourceType, u32)> {
    res.commodity_recipe()
        .and_then(|recipe| recipe.components.into_iter().find(|comp| all.amount(comp.0) < comp.1))
}

fn reaction_first_tier(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    if amount < 5_000 {
        Some(RoomEvent::Request(Request::new(
            RequestKind::Lab(LabData::new(res, max(MIN_LAB_PRODUCTION, 5_000 - amount))),
            Assignment::None,
        )))
    } else if amount > 20_000 {
        //todo lab.reverseReaction(lab1, lab2)
        None
    } else {
        None
    }
}

fn reaction_second_tier(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    (amount < 3_000).then(|| {
        RoomEvent::Request(Request::new(
            RequestKind::Lab(LabData::new(res, max(MIN_LAB_PRODUCTION, 3_000 - amount))),
            Assignment::None,
        ))
    })
}

fn reaction_third_tier(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    if amount < 3_000 {
        //ask colony or craft by itself random
        if game::time().is_multiple_of(200) {
            Some(RoomEvent::Lack(res, 3_000))
        } else {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Lab(LabData::new(res, max(MIN_LAB_PRODUCTION, 3_000 - amount))),
                Assignment::None,
            )))
        }
    } else if amount < 10_000 {
        Some(RoomEvent::Request(Request::new(
            RequestKind::Lab(LabData::new(res, max(MIN_LAB_PRODUCTION, 10_000 - amount))),
            Assignment::None,
        )))
    } else {
        None
    }
}

const fn default_handler(
    _res: ResourceType,
    _amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    None
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
            Assignment::None,
        ))),
        8 if battery > 20_000 => Some(RoomEvent::Excess(ResourceType::Battery, battery - 20_000)),
        7 | 8 if energy < 50_000 && battery >= 50 => Some(RoomEvent::Request(Request::new(
            RequestKind::Factory(FactoryData::new(ResourceType::Energy, 50_000)),
            Assignment::None,
        ))),
        7 | 8 if energy < 50_000 => Some(RoomEvent::Lack(ResourceType::Battery, 5000)),
        6 if energy < 50_000 && ctx.terminal.is_some() => {
            Some(RoomEvent::Lack(ResourceType::Energy, 50_000))
        }
        _ => None,
    }
}

fn battery_handler(
    _res: ResourceType,
    battery: u32,
    resources: &Resources,
    ctx: &RoomContext,
) -> Option<RoomEvent> {
    let energy = resources.amount(ResourceType::Energy);
    if ctx.rcl == 8 && ctx.built_all && battery < 20_000 && energy > 150_000 {
        Some(RoomEvent::Request(Request::new(
            RequestKind::Factory(FactoryData::new(ResourceType::Battery, 1000)),
            Assignment::None,
        )))
    } else {
        None
    }
}

fn power_handler(
    _res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    (amount < 10_000).then(|| RoomEvent::Lack(ResourceType::Power, 10_000))
}

fn ghodium_melt_handler(
    _res: ResourceType,
    ghodium_melt: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    if ghodium_melt < 600 {
        Some(RoomEvent::Request(Request::new(
            RequestKind::Factory(FactoryData::new(ResourceType::GhodiumMelt, 300)),
            Assignment::None,
        )))
    } else {
        None
    }
}

fn compressed_commodities_handler(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    _ctx: &RoomContext,
) -> Option<RoomEvent> {
    // compressed resources are created by mineral handlers except ghodim_melt
    // handler for other rooms this handler throws Lack event
    (amount < 600).then(|| RoomEvent::Lack(res, 600))
}

fn composite_handler(
    res: ResourceType,
    amount: u32,
    _resources: &Resources,
    ctx: &RoomContext,
) -> Option<RoomEvent> {
    if ctx.fl == 1 && amount < 1_000 {
        // craft lvl
        Some(RoomEvent::Request(Request::new(
            RequestKind::Factory(FactoryData::new(res, 600)),
            Assignment::None,
        )))
    } else if amount < 600 {
        Some(RoomEvent::Lack(res, 600 - amount))
    } else {
        None
    }
}

const fn ops_handler(
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
    if let Some(compressed_resource) = get_compressed_resource(res) {
        let compressed_amount = resources.amount(compressed_resource);

        if amount > 50_000 {
            if compressed_amount > 100_000
                && let (Some(t_id), Some(s_id)) = (ctx.terminal, ctx.storage)
            {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Carry(CarryData::new(s_id, t_id, res, amount - 50_000)),
                    Assignment::Single(None),
                )))
            } else {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(compressed_resource, 5_000)),
                    Assignment::None,
                )))
            }
        } else if amount < 5_000 {
            if compressed_amount > 10_000 {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(res, 5_000)),
                    Assignment::None,
                )))
            } else {
                Some(RoomEvent::Lack(res, 5_000 - amount))
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn get_compressed_resource(resource: ResourceType) -> Option<ResourceType> {
    resource.commodity_recipe().and_then(|recipe| {
        recipe.components.iter().find_map(|(component, _)| {
            if *component == ResourceType::Energy { None } else { Some(*component) }
        })
    })
}
