use log::*;
use serde::{Serialize, Deserialize};
use screeps::{Effect, EffectType, HasPosition, MoveToOptions, PowerCreep, PowerInfo,
    PowerType, ResourceType, RoomObjectProperties, SharedCreepProperties, StructureController
};

use crate::{
    rooms::shelter::Shelter,
    utils::constants::LONG_RANGE_ACTION
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PowerCreepMemory {
    #[serde(skip)]
    pub home_room: Option<String>,
    pub trav: Option<TravelData>
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TravelData {
    pub state: Vec<TravelState>,
    pub path: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TravelState {
    IntState(u32),
    StringState(String)
}

pub fn run(creep: &PowerCreep, home: &Shelter) {
    if !home.controller().is_power_enabled() {
        enable_controller(creep, home.controller());
        return;
    }

    if creep.ticks_to_live().is_some_and(|ticks| ticks < 100) {
        renew(creep, home);
        return;
    }

    if is_power_available(creep, PowerType::GenerateOps) {
        let _ = creep.use_power(PowerType::GenerateOps, None);
    }

    if home.invasion() {
        if creep.store().get_used_capacity(Some(ResourceType::Ops)) < 10 {
            if let Some(storage) = home.storage() && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 10 {
                if creep.pos().is_near_to(storage.pos()) {
                    let _ = creep.withdraw(storage, ResourceType::Ops, None);
                } else {
                    let _ = creep.move_to(storage);
                }
            } else {
                warn!("room: {} resource ops not enough!!", home.name());
            }
        } else if is_power_available(creep, PowerType::OperateTower) {
            if let Some(tower) = home.tower_without_effect() {
                if creep.pos().in_range_to(tower.pos(), 3) {
                    let _ = creep.use_power(PowerType::OperateTower, Some(tower));
                } else {
                    let _ = creep.move_to(tower);
                }
            }
            //todo deside use fortify or operate towers firstly?
        } else if is_power_available(creep, PowerType::Fortify) {
            //todo take from room history
            if let Some(rampart) = home.lowest_perimetr_hits() {
                if creep.pos().in_range_to(rampart.pos(), LONG_RANGE_ACTION) {
                    //todo moving safe for powercreep
                    let _ = creep.use_power(PowerType::Fortify, Some(rampart));
                } else {
                    let _ = creep.move_to(rampart);
                }
            }
        }
    } else if let (Some(source), Some(_)) = (home.source_without_effect(), get_power(creep, PowerType::RegenSource)) {
        if creep.pos().in_range_to(source.pos(), 3) {
            let res = creep.use_power(PowerType::RegenSource, Some(source));
            match res {
                Ok(_) => {},
                Err(err) => { error!("use power error: {:?}", err); }
            }
        } else {
            // in_room_move(creep, &source);
            let _ = creep.move_to_with_options(source, Some(MoveToOptions::new().range(LONG_RANGE_ACTION)));
        }
    } else if let (Some(storage), Some(_)) =
        (home.full_storage_without_effect(), get_power(creep, PowerType::OperateStorage))
    {
        if creep.store().get_used_capacity(Some(ResourceType::Ops)) < 100 {
            if let Some(storage) = home.storage() && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 100 {
                if creep.pos().is_near_to(storage.pos()) {
                    let _ = creep.withdraw(storage, ResourceType::Ops, None);
                } else {
                    let _ = creep.move_to(storage);
                }
            } else {
                warn!("room: {} resource ops not enough!!", home.name());
            }
        } else {
            debug!("creep full: {}", creep.name());
            if creep.pos().get_range_to(storage.pos()) <= 3 {
                let res = creep.use_power(PowerType::OperateStorage, Some(storage));
                debug!("creep {} operate storage res: {:?}", creep.name(), res);
                match res {
                    Ok(_) => {},
                    Err(err) => { error!("use power error: {:?}", err); }
                }
            } else {
                let _ = creep.move_to(storage);
            }
        }
    } else if home.mineral_without_effect() && get_power(creep, PowerType::RegenMineral).is_some() {
        if creep.pos().in_range_to(home.mineral().pos(), 3) {
            let res = creep.use_power(PowerType::RegenMineral, Some(home.mineral()));
            match res {
                Ok(_) => {},
                Err(err) => { error!("use power error: {:?}", err); }
            }
        } else {
            let _ = creep.move_to_with_options(home.mineral(), Some(MoveToOptions::new().range(LONG_RANGE_ACTION)));
        }
    } else if let (Some(spawn), Some(_)) = (
        home.spawn_without_effect(),
        get_power(creep, PowerType::OperateSpawn)) && home.is_power_enabled(&PowerType::OperateSpawn)
    {
        if creep.store().get_used_capacity(Some(ResourceType::Ops)) < 100 {
            if let Some(storage) = home.storage() && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 100 {
                if creep.pos().is_near_to(storage.pos()) {
                    let _ = creep.withdraw(storage, ResourceType::Ops, None);
                } else {
                    let _ = creep.move_to(storage);
                }
            } else {
                warn!("room: {} resource ops not enough!!", home.name());
            }
        } else {
            debug!("creep full: {}", creep.name());
            if creep.pos().get_range_to(spawn.pos()) <= 3 {
                let res = creep.use_power(PowerType::OperateSpawn, Some(spawn));
                debug!("creep {} operate spawn res: {:?}", creep.name(), res);
                match res {
                    Ok(_) => {},
                    Err(err) => { error!("use power error: {:?}", err); }
                }
            } else {
                let _ = creep.move_to(spawn);
            }
        }
    } else if let (Some(factory), Some(_), true) = (
        home.factory_without_effect(),
        get_power(creep, PowerType::OperateFactory),
        home.is_power_enabled(&PowerType::OperateFactory))
    {
        if creep.store().get_used_capacity(Some(ResourceType::Ops)) < 100 {
            if let Some(storage) = home.storage() && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 100 {
                if creep.pos().is_near_to(storage.pos()) {
                    let _ = creep.withdraw(storage, ResourceType::Ops, None);
                } else {
                    let _ = creep.move_to(storage);
                }
            } else {
                warn!("room: {} resource ops not enough!!", home.name());
            }
        } else {
            debug!("creep full: {}", creep.name());
            if creep.pos().get_range_to(factory.pos()) <= 3 {
                let res = creep.use_power(PowerType::OperateFactory, Some(factory));
                debug!("creep {} operate storage res: {:?}", creep.name(), res);
                match res {
                    Ok(_) => {},
                    Err(err) => { error!("use power error: {:?}", err); }
                }
            } else {
                let _ = creep.move_to(factory);
            }
        }
    } else if creep.store().get_used_capacity(Some(ResourceType::Ops)) > creep.store().get_capacity(Some(ResourceType::Ops)) / 2 {
        if let Some(storage) = home.storage() && storage.store().get_free_capacity(None) > 5000 {
            if creep.pos().is_near_to(storage.pos()) {
                let _ = creep.transfer(storage, ResourceType::Ops, None);
            } else {
                let _ = creep.move_to(storage);
            }
        } else {
            warn!("room: {}, storage is full!!", home.name())
        }
    } else if controller_without_effect(home.controller())
        && get_power(creep, PowerType::OperateController).is_some()
        && home.is_power_enabled(&PowerType::OperateController)
    {
        if creep.store().get_used_capacity(Some(ResourceType::Ops)) < 200 {
            if let Some(storage) = home.storage() && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 200 {
                if creep.pos().is_near_to(storage.pos()) {
                    let _ = creep.withdraw(storage, ResourceType::Ops, None);
                } else {
                    let _ = creep.move_to(storage);
                }
            } else {
                warn!("room: {} resource ops not enough!!", home.name());
            }
        } else {
            debug!("creep full: {}", creep.name());
            if creep.pos().get_range_to(home.controller().pos()) <= 3 {
                let res = creep.use_power(PowerType::OperateController, Some(home.controller()));
                debug!("creep {} operate controller res: {:?}", creep.name(), res);
                match res {
                    Ok(_) => {},
                    Err(err) => { error!("use power error: {:?}", err); }
                }
            } else {
                let _ = creep.move_to(home.controller());
            }
        }
    } else {
        go_to_workplace(creep, home);
    }
}

fn go_to_workplace(creep: &PowerCreep, home: &Shelter) {
    if let Some(workplace) = home.pc_workplace() {
        if !workplace.is_equal_to(creep.pos()) {
            let _ = creep.move_to(workplace);
        }
    }
}

fn get_power(creep: &PowerCreep, power_type: PowerType) -> Option<PowerInfo> {
    creep.powers().get(power_type)
        .and_then(|p| if p.cooldown() == 0 {
            Some(p)
        } else {
            None
        })
}

fn is_power_available(creep: &PowerCreep, power_type: PowerType) -> bool {
    creep.powers().get(power_type).is_some_and(|power| power.cooldown() == 0)
}

fn controller_without_effect(controller: &StructureController) -> bool {
    !controller.effects().into_iter()
        .any(|effect:Effect| {
            match effect.effect() {
                EffectType::PowerEffect(p) => matches!(p, PowerType::OperateController),
                _ => false
            }
        })
}

fn renew(creep: &PowerCreep, home: &Shelter) {
    if let Some(power_spawn) = home.power_spawn() {
        if creep.pos().is_near_to(power_spawn.pos()) {
            let _ = creep.renew(power_spawn);
        } else {
            let _ = creep.move_to(power_spawn);
        }
    } else {
        warn!("power_creep: {} no powerspawn found for renew!!", creep.name());
    }
}

fn enable_controller(creep: &PowerCreep, controller: &StructureController) {
    if creep.pos().is_near_to(controller.pos()) {
        let _ = creep.enable_room(controller);
    } else {
        let _ = creep.move_to(controller);
    }
}