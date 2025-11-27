use std::cmp;
use screeps::{HasId, ResourceType};
use smallvec::{smallvec, SmallVec};

use crate::{
    commons::get_compressed_resource,
    rooms::{
        RoomEvent, wrappers::claimed::Claimed,
        state::requests::{CarryData, FactoryData, LabData, Request, RequestKind, assignment::Assignment}
    }
};

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

const MIN_LAB_PRODUCTION: u32 = 5;
pub(crate) trait ResourceHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]>;
}

pub(crate) fn handlers(mineral: ResourceType) -> Vec<Box<dyn ResourceHandler>> {
    vec![
        Box::new(MineralHandler(mineral)),
        Box::new(EnergyHandler),
        Box::new(BatteryHandler),
        Box::new(PowerHandler),
        Box::new(OpsHandler),
        // factory common chain
        // factory mechanical chain
        Box::new(MetalHandler),
        Box::new(AlloyHandler),
        Box::new(TubeHandler),
        //factory biological chain
        Box::new(BiomassHandler),
        Box::new(CellHandler),
        Box::new(PhlegmHandler),
        Box::new(TissueHandler),
        Box::new(MuscleHandler),
        Box::new(OrganoidHandler),
        //factory electroinal chain
        Box::new(SiliconHandler),
        Box::new(WireHandler),
        Box::new(SwitchHandler),
        Box::new(TransistorHandler),
        Box::new(MicrochipHandler),
        //factory mystical chain
        Box::new(MistHandler),
        Box::new(CondensateHandler),
        Box::new(ConcentrateHandler),
        Box::new(ExtractHandler),
        Box::new(SpiritHandler),
        //lab 0 tier
        Box::new(HydroxideHandler),
        Box::new(ZynthiumKeaniteHandler),
        Box::new(UtriumLemergiteHandler),
        Box::new(GhodiumHandler),
        //lab 1 tier
        Box::new(UtriumHydrideHandler),
        Box::new(UtriumOxideHandler),
        Box::new(KeaniumHydrideHandler),
        Box::new(KeaniumOxideHandler),
        Box::new(ZynthiumHydrideHandler),
        Box::new(ZynthiumOxideHandler),
        Box::new(LemergiumHydrideHandler),
        Box::new(LemergiumOxideHandler),
        Box::new(GhodiumHydrideHandler),
        // Box::new(GhodiumOxideHandler), //skip, have enough
        //lab 2 tier
        Box::new(UtriumAcidHandler),
        Box::new(UtriumAlkalideHandler),
        Box::new(KeaniumAcidHandler),
        Box::new(KeaniumAlkalideHandler),
        Box::new(LemergiumAcidHandler),
        Box::new(LemergiumAlkalideHandler),
        Box::new(ZynthiumAcidHandler),
        Box::new(ZynthiumAlkalideHandler),
        Box::new(GhodiumAcidHandler),
        Box::new(GhodiumAlkalideHandler),
        //lab 3 tier
        Box::new(CatalyzedGhodiumAcidHandler),
        Box::new(CatalyzedGhodiumAlkalideHandler),
        Box::new(CatalyzedKeaniumAlkalideHandler),
        Box::new(CatalyzedKeaniumAcidHandler), //150 capacity
        Box::new(CatalyzedLemergiumAcidHandler),
        Box::new(CatalyzedLemergiumAlkalideHandler),
        Box::new(CatalyzedUtriumAcidHandler),
        // Box::new(CatalyzedUtriumAlkalideHandler), //+600% harvest effectiveness, don't need?
        Box::new(CatalyzedZynthiumAcidHandler),
        Box::new(CatalyzedZynthiumAlkalideHandler),
    ]
}

struct MineralHandler(ResourceType);
impl ResourceHandler for MineralHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                if storage.store().get_used_capacity(Some(self.0)) > 50000 &&
                        let Some(compressed_resource) = get_compressed_resource(self.0)
                    {
                        if storage.store().get_used_capacity(Some(compressed_resource)) > 100000 &&
                            let Some(terminal) = base.terminal() && terminal.store().get_free_capacity(None) > 50000
                        {
                            Some(RoomEvent::Request(Request::new(
                                RequestKind::Carry(CarryData::new(
                                    storage.raw_id(),
                                    terminal.raw_id(),
                                    self.0,
                                    5000)),
                                Assignment::Single(None))))
                        } else {
                            Some(RoomEvent::Request(Request::new(
                                RequestKind::Factory(FactoryData::new(compressed_resource, 5000)),
                                Assignment::None)))
                        }
                    } else {
                        None
                    }
            })
            .into_iter().collect()
    }
}

struct EnergyHandler;
impl ResourceHandler for EnergyHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let energy = storage.store().get_used_capacity(Some(ResourceType::Energy));
                if base.controller.level() == 8 && energy > 300000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Factory(FactoryData::new(ResourceType::Battery, 5000)),
                        Assignment::None)))
                } else if energy < 50000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Factory(FactoryData::new(ResourceType::Energy, 50000)),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct BatteryHandler;
impl ResourceHandler for BatteryHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::Battery));
                if base.controller.level() < 8 {
                    return None;
                }

                if amount > 50000 {
                    Some(RoomEvent::Excess(ResourceType::Battery, amount - 50000))
                } else if amount < 20000 && storage.store().get_used_capacity(Some(ResourceType::Energy)) > 200000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Factory(FactoryData::new(ResourceType::Battery, 1000)),
                        Assignment::None)))
                } else if amount < 10000 {
                    Some(RoomEvent::Lack(ResourceType::Battery, 10000 - amount))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct PowerHandler;
impl ResourceHandler for PowerHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                if storage.store().get_used_capacity(Some(ResourceType::Power)) < 10000 {
                    Some(RoomEvent::Lack(ResourceType::Power, 3000))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct OpsHandler;
impl ResourceHandler for OpsHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::Ops));
                if amount < 10000 {
                    Some(RoomEvent::Lack(ResourceType::Ops, 3000))
                } else if amount > 100000 {
                    Some(RoomEvent::Excess(ResourceType::Ops, amount - 100000))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct SiliconHandler;
impl ResourceHandler for SiliconHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Silicon)) > 10000)
        {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Factory(FactoryData::new(ResourceType::Wire, 2000)),
                Assignment::None)))
        } else {
            None
        }.into_iter().collect()
    }
}

struct BiomassHandler;
impl ResourceHandler for BiomassHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Biomass)) > 10000)
        {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Factory(FactoryData::new(ResourceType::Cell, 2000)),
                Assignment::None)))
        } else {
            None
        }.into_iter().collect()
    }
}

struct MistHandler;
impl ResourceHandler for MistHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Mist)) > 10000)
        {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Factory(FactoryData::new(ResourceType::Condensate, 2000)),
                Assignment::None)))
        } else {
            None
        }.into_iter().collect()
    }
}

struct CondensateHandler;
impl ResourceHandler for CondensateHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Condensate)) > 3000)
        {
            if base.factory().is_some_and(|f| f.level() == 1) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Concentrate, 300)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Condensate, 3000))
            }
        } else {
           None
        }.into_iter().collect()
    }
}

struct ConcentrateHandler;
impl ResourceHandler for ConcentrateHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Concentrate)) >= 250)
        {
            if base.factory().is_some_and(|f| f.level() == 2) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Extract, 50)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Concentrate, 250))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct ExtractHandler;
impl ResourceHandler for ExtractHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Extract)) >= 40)
        {
            if base.factory().is_some_and(|f| f.level() == 3) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Spirit, 20)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Extract, 40))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct SpiritHandler;
impl ResourceHandler for SpiritHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Spirit)) >= 16)
        {
            if base.factory().is_some_and(|f| f.level() == 4) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Emanation, 8)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Spirit, 16))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct MetalHandler;
impl ResourceHandler for MetalHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Metal)) > 10000)
        {
            Some(RoomEvent::Request(Request::new(
                RequestKind::Factory(FactoryData::new(ResourceType::Alloy, 2000)),
                Assignment::None)))
        } else {
            None
        }.into_iter().collect()
    }
}

struct AlloyHandler;
impl ResourceHandler for AlloyHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Alloy)) > 3000)
        {
            if base.factory().is_some_and(|f| f.level() == 1) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Tube, 150)),
                    Assignment::None)))
            } else if base.factory().is_some_and(|f| f.level() == 2) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Fixtures, 70)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Alloy, 3000))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct TubeHandler;
impl ResourceHandler for TubeHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Tube)) >= 40)
        {
            if base.factory().is_some_and(|f| f.level() == 3) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Frame, 10)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Tube, 40))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct WireHandler;
impl ResourceHandler for WireHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]>{
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Wire)) > 3000)
        {
            if base.factory().is_some_and(|f| f.level() == 1) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Switch, 375)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Wire, 3000))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct SwitchHandler;
impl ResourceHandler for SwitchHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Switch)) >= 400)
        {
            if base.factory().is_some_and(|f| f.level() == 2) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Transistor, 100)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Switch, 400))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct TransistorHandler;
impl ResourceHandler for TransistorHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Transistor)) >= 50)
        {
            if base.factory().is_some_and(|f| f.level() == 3) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Microchip, 25)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Transistor, 50))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct MicrochipHandler;
impl ResourceHandler for MicrochipHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Microchip)) >= 25)
        {
            Some(RoomEvent::Excess(ResourceType::Microchip, 25))
        } else {
            None
        }.into_iter().collect()
    }
}

struct CellHandler;
impl ResourceHandler for CellHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Cell)) > 3000)
        {
            if base.factory().is_some_and(|f| f.level() == 1) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Phlegm, 300)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Cell, 3000))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct PhlegmHandler;
impl ResourceHandler for PhlegmHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Phlegm)) >= 400)
        {
            if base.factory().is_some_and(|f| f.level() == 2) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Tissue, 80)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Phlegm, 400))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct TissueHandler;
impl ResourceHandler for TissueHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Tissue)) >= 99)
        {
            if base.factory().is_some_and(|f| f.level() == 3) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Muscle, 33)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Tissue, 99))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct MuscleHandler;
impl ResourceHandler for MuscleHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Muscle)) >= 10)
        {
            if base.factory().is_some_and(|f| f.level() == 4) {
                Some(RoomEvent::Request(Request::new(
                    RequestKind::Factory(FactoryData::new(ResourceType::Organoid, 10)),
                    Assignment::None)))
            } else {
                Some(RoomEvent::Excess(ResourceType::Muscle, 10))
            }
        } else {
            None
        }.into_iter().collect()
    }
}

struct OrganoidHandler;
impl ResourceHandler for OrganoidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        if base.storage()
            .is_some_and(|storage|
                storage.store().get_used_capacity(Some(ResourceType::Organoid)) >= 10)
        {
            Some(RoomEvent::Excess(ResourceType::Organoid, 10))
        } else {
            None
        }.into_iter().collect()
    }
}

//production resources
//0 tier
struct HydroxideHandler;
impl ResourceHandler for HydroxideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::Hydroxide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::Hydroxide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct ZynthiumKeaniteHandler;
impl ResourceHandler for ZynthiumKeaniteHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::ZynthiumKeanite));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::ZynthiumKeanite,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct UtriumLemergiteHandler;
impl ResourceHandler for UtriumLemergiteHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::UtriumLemergite));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::UtriumLemergite,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct GhodiumHandler;
impl ResourceHandler for GhodiumHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::Ghodium));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::Ghodium,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

//1 tier
struct UtriumHydrideHandler;
impl ResourceHandler for UtriumHydrideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::UtriumHydride));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::UtriumHydride,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct UtriumOxideHandler;
impl ResourceHandler for UtriumOxideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::UtriumOxide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::UtriumOxide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct KeaniumHydrideHandler;
impl ResourceHandler for KeaniumHydrideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::KeaniumHydride));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::KeaniumHydride,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct KeaniumOxideHandler;
impl ResourceHandler for KeaniumOxideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::KeaniumOxide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::KeaniumOxide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct LemergiumHydrideHandler;
impl ResourceHandler for LemergiumHydrideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::LemergiumHydride));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::LemergiumHydride,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct LemergiumOxideHandler;
impl ResourceHandler for LemergiumOxideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::LemergiumOxide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::LemergiumOxide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct ZynthiumHydrideHandler;
impl ResourceHandler for ZynthiumHydrideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::ZynthiumHydride));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::ZynthiumHydride,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct ZynthiumOxideHandler;
impl ResourceHandler for ZynthiumOxideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::ZynthiumOxide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::ZynthiumOxide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct GhodiumHydrideHandler;
impl ResourceHandler for GhodiumHydrideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::GhodiumHydride));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::GhodiumHydride,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct GhodiumOxideHandler;
impl ResourceHandler for GhodiumOxideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::GhodiumOxide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::GhodiumOxide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else if amount > 10000 {
                    //todo lab.reverseReaction(lab1, lab2)
                    None
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

//2 tier
struct UtriumAcidHandler;
impl ResourceHandler for UtriumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::UtriumAcid));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::UtriumAcid,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct UtriumAlkalideHandler;
impl ResourceHandler for UtriumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::UtriumAlkalide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::UtriumAlkalide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct KeaniumAcidHandler;
impl ResourceHandler for KeaniumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::KeaniumAcid));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::KeaniumAcid,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct KeaniumAlkalideHandler;
impl ResourceHandler for KeaniumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::KeaniumAlkalide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::KeaniumAlkalide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct LemergiumAcidHandler;
impl ResourceHandler for LemergiumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::LemergiumAcid));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::LemergiumAcid,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct LemergiumAlkalideHandler;
impl ResourceHandler for LemergiumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::LemergiumAlkalide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::LemergiumAlkalide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct ZynthiumAcidHandler;
impl ResourceHandler for ZynthiumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::ZynthiumAcid));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::ZynthiumAcid,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}
struct ZynthiumAlkalideHandler;
impl ResourceHandler for ZynthiumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::ZynthiumAlkalide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::ZynthiumAlkalide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct GhodiumAcidHandler;
impl ResourceHandler for GhodiumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::GhodiumAcid));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::GhodiumAcid,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

struct GhodiumAlkalideHandler;
impl ResourceHandler for GhodiumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::GhodiumAlkalide));
                if amount < 3000 {
                    Some(RoomEvent::Request(Request::new(
                        RequestKind::Lab(LabData::new(
                            ResourceType::GhodiumAlkalide,
                            cmp::max(MIN_LAB_PRODUCTION, 3000 - amount))),
                        Assignment::None)))
                } else {
                    None
                }
            })
            .into_iter().collect()
    }
}

//3 tier
struct CatalyzedUtriumAcidHandler;
impl ResourceHandler for CatalyzedUtriumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedUtriumAcid));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedUtriumAcid, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedUtriumAcid,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedUtriumAcid,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedUtriumAlkalideHandler;
impl ResourceHandler for CatalyzedUtriumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedUtriumAlkalide));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedUtriumAlkalide, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedUtriumAlkalide,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedUtriumAlkalide,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedKeaniumAcidHandler;
impl ResourceHandler for CatalyzedKeaniumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedKeaniumAcid));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedKeaniumAcid, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedKeaniumAcid,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedKeaniumAcid,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedKeaniumAlkalideHandler;
impl ResourceHandler for CatalyzedKeaniumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedKeaniumAlkalide));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedKeaniumAlkalide, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedKeaniumAlkalide,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedKeaniumAlkalide,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedLemergiumAcidHandler;
impl ResourceHandler for CatalyzedLemergiumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedLemergiumAcid));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedLemergiumAcid, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedLemergiumAcid,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedLemergiumAcid,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedLemergiumAlkalideHandler;
impl ResourceHandler for CatalyzedLemergiumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedLemergiumAlkalide));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedLemergiumAlkalide, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedLemergiumAlkalide,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedLemergiumAlkalide,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedZynthiumAcidHandler;
impl ResourceHandler for CatalyzedZynthiumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedZynthiumAcid));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedZynthiumAcid, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedZynthiumAcid,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedZynthiumAcid,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedZynthiumAlkalideHandler;
impl ResourceHandler for CatalyzedZynthiumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedZynthiumAlkalide));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedZynthiumAlkalide, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedZynthiumAlkalide,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedZynthiumAlkalide,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedGhodiumAlkalideHandler;
impl ResourceHandler for CatalyzedGhodiumAlkalideHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAlkalide));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedGhodiumAlkalide, 3000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedGhodiumAlkalide,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedGhodiumAlkalide,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}

struct CatalyzedGhodiumAcidHandler;
impl ResourceHandler for CatalyzedGhodiumAcidHandler {
    fn handle(&self, base: &Claimed) -> SmallVec<[RoomEvent; 2]> {
        base.storage()
            .and_then(|storage| {
                let amount = storage.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAcid));
                if amount < 3000 {
                    Some(smallvec![
                        RoomEvent::Lack(ResourceType::CatalyzedGhodiumAcid, 1000),
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedGhodiumAcid,
                                3000)),
                            Assignment::None))
                    ])
                } else if amount < 10000 {
                    Some(smallvec![
                        RoomEvent::Request(Request::new(
                            RequestKind::Lab(LabData::new(
                                ResourceType::CatalyzedGhodiumAcid,
                                cmp::max(MIN_LAB_PRODUCTION, 10000 - amount))),
                            Assignment::None))
                    ])
                } else {
                    None
                }
            }).unwrap_or_default()
    }
}
