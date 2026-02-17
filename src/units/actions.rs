use log::{debug, warn, error};
use std::sync::Arc;

use screeps::{Effect, EffectType, HasPosition, Position, PowerType, ResourceType, RoomObjectProperties, Source, StructureController, StructureFactory, StructureSpawn, StructureStorage, StructureTower};
use crate::{units::power_creep::PcUnit, utils::constants::{CLOSE_RANGE_ACTION, LONG_RANGE_ACTION, MIN_STORAGE_FREE_CAPACITY}};

const MIN_TICKS: u32 = 100;

type PowerAction = Arc<dyn Fn(&PcUnit) -> Option<Goal> + Send + Sync>;
type Goal = (Position, u32);

pub fn common_actions(tail: PowerAction) -> PowerAction {
    enable_controller(
        renew(
            generate_ops(tail)
        )
    )
}

pub fn end_of_chain() -> PowerAction {
    Arc::new(|_| None) // end of chain
}

fn enable_controller(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        let controller = unit.home_controller();

        if !controller.is_power_enabled()
        {
            if unit.pos().is_near_to(controller.pos()) {
                let _ = unit.enable_room(controller);
                return None;
            } else {
                return Some((controller.pos(), CLOSE_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn fortify(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if unit.is_power_available(PowerType::Fortify) {
            //todo take from room history
            if let Some(rampart) = unit.home_lowest_perimeter() {
                if unit.pos().in_range_to(rampart.pos(), LONG_RANGE_ACTION) {
                    //todo moving safe for powercreep
                    let _ = unit.use_power(PowerType::Fortify, Some(rampart));
                    return None;
                } else {
                    return Some((rampart.pos(), LONG_RANGE_ACTION));
                }
            } else {
                return None;
            }
        }
        next(unit)
    })
}

fn generate_ops(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if unit.is_power_available(PowerType::GenerateOps) {
            let _ = unit.use_power(PowerType::GenerateOps, None);
            return None;
        }
        next(unit)
    })
}

pub fn operate_controller(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        let controller = unit.home_controller();
        if controller_without_effect(controller)
            && unit.get_power(PowerType::OperateController).is_some()
            && unit.is_power_enabled(PowerType::OperateController)
        {
            if unit.pos().get_range_to(controller.pos()) <= LONG_RANGE_ACTION
            {
                let res = unit.use_power(PowerType::OperateController, Some(controller));
                debug!("creep {} operate controller res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                return None;
            } else {
                return Some((controller.pos(), LONG_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn operate_factory(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if let (Some(factory), Some(_), true) = (
            factory_without_effect(unit),
            unit.get_power(PowerType::OperateFactory),
            unit.is_power_enabled(PowerType::OperateFactory),
        ) {
            if unit.pos().get_range_to(factory.pos()) <= LONG_RANGE_ACTION {
                let res = unit.use_power(PowerType::OperateFactory, Some(factory));
                debug!("creep {} operate storage res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                return None;
            } else {
                return Some((factory.pos(), LONG_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn operate_mineral(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        let mineral = unit.home_mineral();
        if mineral_without_effect(unit) && unit.get_power(PowerType::RegenMineral).is_some()
        {
            if unit.pos().in_range_to(mineral.pos(), 3) {
                let res = unit.use_power(PowerType::RegenMineral, Some(mineral));
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                return None;
            } else {
                return Some((mineral.pos(), LONG_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn operate_source(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if let (Some(source), Some(_)) =
            (source_without_effect(unit), unit.get_power(PowerType::RegenSource))
        {
            if unit.pos().in_range_to(source.pos(), LONG_RANGE_ACTION) {
                let res = unit.use_power(PowerType::RegenSource, Some(source));
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                return None;
            } else {
                return Some((source.pos(), LONG_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn operate_spawn(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if let (Some(spawn), Some(_)) =
            (spawn_without_effect(unit), unit.get_power(PowerType::OperateSpawn))
            && unit.is_power_enabled(PowerType::OperateSpawn)
        {
            if unit.pos().get_range_to(spawn.pos()) <= LONG_RANGE_ACTION {
                let res = unit.use_power(PowerType::OperateSpawn, Some(spawn));
                debug!("creep {} operate spawn res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                return None;
            } else {
                return Some((spawn.pos(), LONG_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn operate_storage(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if let (Some(storage), Some(_)) =
            (full_storage_without_effect(unit), unit.get_power(PowerType::OperateStorage))
        {
            if unit.pos().get_range_to(storage.pos()) <= LONG_RANGE_ACTION {
                let res = unit.use_power(PowerType::OperateStorage, Some(storage));
                debug!("creep {} operate storage res: {:?}", unit.name(), res);
                match res {
                    Ok(()) => {}
                    Err(err) => error!("use power error: {:?}", err)
                }
                return None;
            } else {
                return Some((storage.pos(), LONG_RANGE_ACTION));
            }
        }
        next(unit)
    })
}

pub fn operate_tower(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if unit.is_power_available(PowerType::OperateTower) {
            if let Some(tower) = tower_without_effect(unit) {
                if unit.pos().in_range_to(tower.pos(), LONG_RANGE_ACTION) {
                    let _ = unit.use_power(PowerType::OperateTower, Some(tower));
                    return None;
                } else {
                    return Some((tower.pos(), LONG_RANGE_ACTION));
                }
            } else {
                return None;
            }
        }
        next(unit)
    })
}

fn renew(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if unit.ticks_to_live().is_some_and(|ticks| ticks < MIN_TICKS)
        {
            if let Some(power_spawn) = unit.home_power_spawn() {
                if unit.pos().is_near_to(power_spawn.pos()) {
                    let _ = unit.renew(power_spawn);
                    return None;
                } else {
                    return Some((power_spawn.pos(), CLOSE_RANGE_ACTION));
                }
            } else {
                warn!("power_creep: {} no powerspawn found for renew!!", unit.name());
                return None;
            }
        }
        next(unit)
    })
}

pub fn transfer(next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if unit.used_capacity(Some(ResourceType::Ops)) * 10 >= unit.capacity() * 9
        {
            if let Some(storage) = unit.home_storage()
                && storage.store().get_free_capacity(None) > MIN_STORAGE_FREE_CAPACITY
            {
                if unit.pos().is_near_to(storage.pos()) {
                    let _ = unit.transfer(storage, ResourceType::Ops, None);
                    return None;
                } else {
                    return Some((storage.pos(), CLOSE_RANGE_ACTION));
                }
            } else {
                warn!("room: {}, storage is full!!", unit.home_name());
                return None;
            }
        }
        next(unit)
    })
}

pub fn withdraw(limit: u32, next: PowerAction) -> PowerAction {
    Arc::new(move |unit| {
        if unit.used_capacity(Some(ResourceType::Ops)) < limit {
            if let Some(storage) = unit.home_storage()
                && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= limit
            {
                if unit.pos().is_near_to(storage.pos()) {
                    let _ = unit.withdraw(storage, ResourceType::Ops, Some(unit.capacity() / 2));
                    return None;
                } else {
                    return Some((storage.pos(), CLOSE_RANGE_ACTION));
                }
            } else {
                warn!("room: {} resource ops not enough!!", unit.home_name());
                return None;
            }
        }
        next(unit)
    })
}

fn full_storage_without_effect<'a>(unit: &'a PcUnit) -> Option<&'a StructureStorage> {
    unit.home_storage().filter(|storage| {
        storage.effects().is_empty() && storage.store().get_used_capacity(None) > 990_000
    })
}

fn factory_without_effect<'a>(unit: &'a PcUnit) -> Option<&'a StructureFactory> {
    unit.home_factory().filter(|factory| {
        !factory.effects().into_iter().any(|effect: Effect| match effect.effect() {
            EffectType::PowerEffect(p) => matches!(p, PowerType::OperateFactory),
            EffectType::NaturalEffect(_) => false,
        })
    })
}

fn mineral_without_effect(unit: &PcUnit) -> bool {
    unit.home_mineral().ticks_to_regeneration().is_none()
        && !unit.home_mineral().effects().into_iter().any(|effect: Effect| {
            match effect.effect() {
                EffectType::PowerEffect(p) => matches!(p, PowerType::RegenMineral),
                EffectType::NaturalEffect(_) => false,
            }
        })
}

fn source_without_effect<'a>(unit: &'a PcUnit) -> Option<&'a Source> {
    //todo check remote rooms sources for powers without hardcoded ids
    unit.home_sources().iter().find(|source| {
        !source.effects().into_iter().any(|effect: Effect| match effect.effect() {
            EffectType::PowerEffect(p) => {
                matches!(p, PowerType::RegenSource if { effect.ticks_remaining() > 30 })
            }
            EffectType::NaturalEffect(_) => false,
        })
    })
}

fn controller_without_effect(controller: &StructureController) -> bool {
    !controller.effects().into_iter().any(|effect: Effect| match effect.effect() {
        EffectType::PowerEffect(p) => matches!(p, PowerType::OperateController),
        EffectType::NaturalEffect(_) => false,
    })
}

fn spawn_without_effect<'a>(unit: &'a PcUnit) -> Option<&'a StructureSpawn> {
    unit.home_spawns().iter().find(|spawn| {
        !spawn.effects().into_iter().any(|effect: Effect| match effect.effect() {
            EffectType::PowerEffect(p) => matches!(p, PowerType::OperateSpawn),
            EffectType::NaturalEffect(_) => false,
        })
    })
}

fn tower_without_effect<'a>(unit: &'a PcUnit) -> Option<&'a StructureTower> {
    unit.home_towers().iter().find(|tower| {
        !tower.effects().into_iter().any(|effect: Effect| match effect.effect() {
            EffectType::PowerEffect(p) => matches!(p, PowerType::OperateTower),
            EffectType::NaturalEffect(_) => false,
        })
    })
}