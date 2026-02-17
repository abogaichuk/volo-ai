use screeps::{Effect, EffectType, PowerType, RoomObjectProperties, Source, StructureFactory, StructureSpawn, StructureStorage, StructureTower};

use crate::{movement::MovementGoal, units::{actions::{enable_controller::EnableController, fortify::Fortify, generate_ops::GenerateOps, operate_controller::OperateController, operate_factory::OperateFactory, operate_mineral::OperateMineral, operate_source::OperateSource, operate_spawn::OperateSpawn, operate_storage::OperateStorage, operate_tower::OperateTower, renew::Renew, transfer::Transfer, withdraw::Withdraw}, power_creep::PcUnit}};

mod withdraw;
mod transfer;
mod renew;
mod generate_ops;
mod enable_controller;
mod operate_tower;
mod operate_source;
mod operate_mineral;
mod operate_storage;
mod operate_spawn;
mod operate_factory;
mod operate_controller;
mod fortify;


type ActionFn = Box<dyn Fn(&PcUnit) -> Option<MovementGoal> + Send + Sync>;

fn chain_fns(steps: Vec<ActionFn>) -> ActionFn {
    Box::new(move |pc| {
        for f in &steps {
            if let Some(goal) = f(pc) {
                return Some(goal);
            }
        }
        None
    })
}

pub trait Action {
    fn handle(&self, pc: &PcUnit) -> Option<MovementGoal>;
    fn next(&self) -> &Option<Box<dyn Action>>;
}

pub fn power_actions(unit: &PcUnit) -> Box<dyn Action> {
    let hostiles = unit.get_hostiles_at_home();

    if unit.is_home_invaded() {
        Box::new(EnableController::new(Renew::new(GenerateOps::new(Withdraw::new(OperateTower::new(Fortify::default()), 10)))))
    } else if !hostiles.is_empty() {
        Box::new(EnableController::new(Renew::new(GenerateOps::new(Withdraw::new(
            OperateStorage::new(
                OperateTower::new(
                    OperateSource::new(
                        OperateMineral::default()))),
            100)))))
    } else {
        Box::new(EnableController::new(Renew::new(GenerateOps::new(OperateSource::new(
            Withdraw::new(
                OperateStorage::new(
                    OperateMineral::new(
                        OperateSpawn::new(
                            OperateFactory::new(
                                OperateController::new(
                                    Transfer::default()))))),
            200))))))
    }
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