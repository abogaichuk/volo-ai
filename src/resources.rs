use std::{cmp, collections::HashMap};
use log::info;
use screeps::{HasId, ObjectId, RawObjectId, ResourceType, StructureTerminal};
use smallvec::{smallvec, SmallVec};

use crate::{
    commons::get_compressed_resource, resources::handlers::{ResourceHandlers, room_handler_for}, rooms::{
        RoomEvent, state::requests::{CarryData, FactoryData, LabData, Request, RequestKind, assignment::Assignment}, wrappers::claimed::Claimed
    }
};

// mod policy;
mod handlers;
mod chain_config;

const MIN_LAB_PRODUCTION: u32 = 5;

pub struct RoomContext {
    pub rcl: u8,
    pub terminal: Option<RawObjectId>,
    pub storage: Option<RawObjectId>,
    pub fl: u8
}

impl RoomContext {
    pub fn new(rcl: u8, terminal: Option<RawObjectId>, storage: Option<RawObjectId>, fl: u8) -> Self {
        Self { rcl, terminal, storage, fl }
    }
}

pub struct Resources {
    amounts: HashMap<ResourceType, u32>
}

impl Resources {
    pub fn new(amounts: HashMap<ResourceType, u32>) -> Self {
        Self { amounts }
    }

    pub fn amount(&self, res: ResourceType) -> u32 {
        *self.amounts.get(&res).unwrap_or(&0)
    }

    pub fn events<'a>(
        &'a self,
        ctx: RoomContext,
    ) -> impl Iterator<Item = RoomEvent> + 'a {
        self.amounts.iter()
            .filter_map(move |(res, amount)| room_handler_for(*res) (*res, *amount, self, &ctx))
    }
}


bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
    pub struct Kinds: u32 {
        const MINERAL   = 1 << 0;
        const PRODUCEABLE = 1 << 1;
        const TRADEABLE = 1 << 2;
        const STOREABLE = 1 << 4;
    }
}

pub fn kinds(rt: ResourceType) -> Kinds {
    match rt {
        ResourceType::Keanium | ResourceType::Utrium | ResourceType::Zynthium |
            ResourceType::Catalyst | ResourceType::Hydrogen | ResourceType::Oxygen |
            ResourceType::Lemergium => Kinds::MINERAL | Kinds::STOREABLE,

        // 0 factory lvl
        ResourceType::UtriumBar | ResourceType::LemergiumBar | ResourceType::ZynthiumBar | ResourceType::KeaniumBar |
        ResourceType::Oxidant | ResourceType::Reductant | ResourceType::Purifier | ResourceType::GhodiumMelt |
        ResourceType::Wire | ResourceType::Cell | ResourceType::Alloy | ResourceType::Condensate |
        // 1 factory lvl
        ResourceType::Composite | ResourceType::Tube | ResourceType::Phlegm | ResourceType::Switch | ResourceType::Concentrate |
        //2 factory lvl
        ResourceType::Crystal | ResourceType::Fixtures | ResourceType::Tissue | ResourceType::Transistor | ResourceType::Extract |
        //3 factory lvl
        ResourceType::Liquid | ResourceType::Frame | ResourceType::Muscle | ResourceType::Spirit |
        //4 factory lvl
        ResourceType::Hydraulics | ResourceType::Circuit => Kinds::PRODUCEABLE,


        ResourceType::Microchip | ResourceType::Organoid | ResourceType::Emanation | ResourceType::Ops => Kinds::TRADEABLE,
        _ => Kinds::STOREABLE
    }
}