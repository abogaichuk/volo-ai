use log::warn;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::{
    Creep, HasHits, HasId, HasPosition, Mineral, ObjectId, Position, ResourceType,
    SharedCreepProperties, Source, StructureController, StructureObject, StructureType, find, look,
};

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::tasks::FLEE_RANGE;
use crate::units::{Task, TaskResult, need_escape};
use crate::utils::commons::{
    closest_attacker, find_closest_exit, find_container_near_by, find_cs_near_by, has_enough_space,
};

pub fn harvest_energy_forever(
    workplace: Position,
    id: ObjectId<Source>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if role.get_home().is_some_and(|home| *home != workplace.room_name()) && need_escape(&enemies) {
        transfer_or_drop(workplace, creep);
        if let Some(closest_exit) = find_closest_exit(creep, None) {
            let goal = Walker::Exploring(false).walk(closest_exit, 0, creep, role, enemies);
            TaskResult::StillWorking(Task::Escape(closest_exit), Some(goal))
        } else {
            warn!("{} no exit found in room {}", creep.name(), creep.pos().room_name());
            TaskResult::Abort
        }
    } else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
        .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 4)
    {
        //flee
        transfer_or_drop(workplace, creep);
        let goal = Walker::Flee.walk(in_range_attacker.pos(), FLEE_RANGE, creep, role, enemies);
        TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
    } else if !creep.pos().is_equal_to(workplace) {
        //no enemies near, go to the workplace
        let goal = Walker::Exploring(false).walk(workplace, 0, creep, role, enemies);
        TaskResult::StillWorking(Task::HarvestEnergyForever(workplace, id), Some(goal))
    } else if let Some(source) = id.resolve() {
        //no enemies near
        if creep.store().get_used_capacity(Some(ResourceType::Energy)) > half_capacity(creep) {
            if let Some(link) = workplace
                .find_in_range(find::MY_STRUCTURES, 1)
                .into_iter()
                .find_map(|structure| match structure {
                    StructureObject::StructureLink(l)
                        if l.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                    {
                        Some(l)
                    }
                    _ => None,
                })
            {
                let _ = creep.say("⛏︎", false);
                let _ = creep.harvest(&source);
                let _ = creep.transfer(&link, ResourceType::Energy, None);
            } else if let Some(container) =
                workplace.look_for(look::STRUCTURES).ok().and_then(|structures| {
                    structures.into_iter().find_map(|structure| match structure {
                        StructureObject::StructureContainer(c) => Some(c),
                        _ => None,
                    })
                })
            {
                if container.hits() < container.hits_max() - 25000 {
                    let _ = creep.say("🛠︎", false);
                    let _ = creep.repair(&container);
                } else {
                    let _ = creep.say("⛏︎", false);
                    let _ = creep.harvest(&source);
                    let _ = creep.transfer(&container, ResourceType::Energy, None);
                }
            } else if let Some(cs) = find_cs_near_by(&workplace, 3) {
                let _ = creep.say("🛠︎", false);
                let _ = creep.build(&cs);
            } else {
                // proceed harvesting dropping resource to the ground
                let _ = creep.say("⛏︎", false);
                let _ = creep.harvest(&source);
            }
            TaskResult::StillWorking(Task::HarvestEnergyForever(workplace, id), None)
        } else {
            let _ = creep.say("⛏︎", false);
            let _ = creep.harvest(&source);
            TaskResult::StillWorking(Task::HarvestEnergyForever(workplace, id), None)
        }
    } else {
        let _ = creep.say("🚬", false);
        TaskResult::StillWorking(Task::Idle(1), None)
    }
}

fn half_capacity(creep: &Creep) -> u32 {
    let capcity = creep.store().get_capacity(None);
    if capcity > 0 { capcity / 2 } else { 0 }
}

pub fn harvest_minerals(
    workplace: Position,
    id: ObjectId<Mineral>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if need_escape(&enemies) {
        //escape
        if let Some(closest_exit) = find_closest_exit(creep, None) {
            let goal = Walker::Exploring(false).walk(closest_exit, 0, creep, role, enemies);
            TaskResult::StillWorking(Task::Escape(closest_exit), Some(goal))
        } else {
            warn!("{} no exit found in room {}", creep.name(), creep.pos().room_name());
            TaskResult::Abort
        }
    } else if let Some(in_range_attacker) = closest_attacker(creep, enemies.iter())
        .filter(|enemy| enemy.pos().get_range_to(creep.pos()) <= 4)
    {
        //flee
        let goal = Walker::Flee.walk(in_range_attacker.pos(), FLEE_RANGE, creep, role, enemies);
        TaskResult::StillWorking(Task::Flee(FLEE_RANGE), Some(goal))
    } else if !creep.pos().is_equal_to(workplace) {
        let goal = Walker::Exploring(false).walk(workplace, 0, creep, role, enemies);
        TaskResult::StillWorking(Task::HarvestMineral(workplace, id), Some(goal))
    } else if let Some(mineral) = id.resolve() {
        if mineral.ticks_to_regeneration() > creep.ticks_to_live() {
            TaskResult::Suicide
        } else {
            let _ = creep.say("⛏︎", false);
            let _ = creep.harvest(&mineral);
            TaskResult::StillWorking(Task::HarvestMineral(workplace, mineral.id()), None)
        }
    } else {
        warn!("{} invalid mineral {} near: {}", creep.name(), id, workplace);
        TaskResult::Abort
    }
}

pub fn harvest_until_full(
    workplace: Position,
    id: ObjectId<Source>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if !creep.pos().is_equal_to(workplace) {
        let goal = Walker::Exploring(false).walk(workplace, 0, creep, role, enemies);
        TaskResult::StillWorking(Task::Harvest(workplace, id), Some(goal))
    } else if !is_full(creep) {
        if let Some(source) = id.resolve() {
            let _ = creep.say("⛏︎", false);
            match creep.harvest(&source) {
                Ok(()) => TaskResult::StillWorking(Task::Harvest(workplace, id), None),
                Err(err) => match err {
                    HarvestErrorCode::NotEnoughResources if better_work(&source, creep) => {
                        TaskResult::Abort
                    }
                    _ => TaskResult::StillWorking(Task::Harvest(workplace, id), None),
                },
            }
            // TaskResult::StillWorking(Task::Harvest(workplace, id), None)
        } else {
            warn!(
                "{} source {} near: {} can't be resolved!",
                creep.name(),
                id.to_string(),
                workplace
            );
            TaskResult::Abort
        }
    } else {
        TaskResult::Completed
    }
}

fn better_work(source: &Source, creep: &Creep) -> bool {
    if let Some(ticks) = source.ticks_to_regeneration()
        && (ticks > 100
            || ticks > 30
                && creep.store().get_used_capacity(Some(ResourceType::Energy))
                    > creep.store().get_capacity(None) / 2)
    {
        true
    } else {
        false
    }
}

pub fn harvest_and_upgrade(
    workplace: Position,
    id: ObjectId<Source>,
    ctrl_id: ObjectId<StructureController>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if !creep.pos().is_equal_to(workplace) {
        let goal = Walker::Exploring(false).walk(workplace, 0, creep, role, enemies);
        TaskResult::StillWorking(Task::HarvestAndUpgrade(workplace, id, ctrl_id), Some(goal))
    } else if !is_full(creep) {
        if let Some(source) = id.resolve() {
            let _ = creep.say("⛏︎", false);
            let _ = creep.harvest(&source);

            if let Some(controller) = ctrl_id.resolve() {
                let _ = creep.upgrade_controller(&controller);
            }

            TaskResult::StillWorking(Task::HarvestAndUpgrade(workplace, id, ctrl_id), None)
        } else {
            warn!(
                "{} source {} near: {} can't be resolved!",
                creep.name(),
                id.to_string(),
                workplace
            );
            TaskResult::Abort
        }
    } else {
        TaskResult::Completed
    }
}

fn is_full(creep: &Creep) -> bool {
    creep.store().get_used_capacity(None)
        >= creep.store().get_capacity(None) - creep.store().get_capacity(None) / 25 // 4%
}

fn transfer_or_drop(workplace: Position, creep: &Creep) {
    if let Some(resource) = creep.store().store_types().first() {
        match find_container_near_by(
            &workplace,
            1,
            &[StructureType::Link, StructureType::Container],
        ) {
            Some(StructureObject::StructureContainer(c))
                if has_enough_space(&c, creep.store().get_used_capacity(Some(*resource))) =>
            {
                let _ = creep.transfer(&c, *resource, None);
            }
            Some(StructureObject::StructureLink(l))
                if *resource == ResourceType::Energy
                    && has_enough_space(
                        &l,
                        creep.store().get_used_capacity(Some(ResourceType::Energy)),
                    ) =>
            {
                let _ = creep.transfer(&l, ResourceType::Energy, None);
            }
            _ => {
                let _ = creep.drop(*resource, None);
            }
        }
    }
}
