use std::collections::HashSet;

use screeps::{Creep, game, HasPosition, SharedCreepProperties};

use crate::{movement::walker::Walker, units::{roles::Role, tasks::{Task, TaskResult}}};

pub fn heal(
    members: HashSet<String>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if let Some(member) = members
        .iter()
        .find(|name| **name != creep.name())
        .and_then(|member| game::creeps().get(member.clone()))
    {
        let injured = [creep, &member].into_iter()
            .min_by_key(|c| c.hits()).unwrap_or_else(|| creep);
        match creep.pos().get_range_to(injured.pos()) {
            0 | 1 => {
                let _ = creep.heal(injured);
                let _ = creep.ranged_mass_attack();
            },
            2 | 3 => { let _ = creep.ranged_heal(injured); },
            _ => {
                let _ = creep.heal(creep);
                let _ = creep.ranged_mass_attack();
            }
        }

        let goal = Walker::Exploring(true).walk(member.pos(), 0, creep, role, enemies);
        TaskResult::StillWorking(Task::Heal(members), Some(goal))
    } else {
        //wait for member getting fresh members on the next tick
        let _ = creep.heal(creep);
        TaskResult::Completed
    }
}