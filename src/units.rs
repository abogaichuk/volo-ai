use thiserror::Error;
use screeps::{Creep, Part, SOURCE_KEEPER_USERNAME};
use crate::{commons::has_part, movement::MovementGoal};
use self::{roles::Role, tasks::{Task, TaskResult}};

pub mod tasks;
pub mod roles;
pub mod power_creep;
pub mod creeps;


#[derive(Error, Debug, Eq, PartialEq)]
pub enum UnitError {
    #[error("creep home room is not set")]
    HomeRoomIsNotSet,
}

fn with_parts(enemies: Vec<Creep>, parts: Vec<Part>) -> Vec<Creep> {
    enemies.into_iter()
        .filter(|creep| has_part(&parts, creep, true))
        .collect()
}

fn need_escape(enemies: &[Creep]) -> bool {
    enemies.iter()
        .any(|hostile| hostile.owner().username() != SOURCE_KEEPER_USERNAME &&
            has_part(&[Part::RangedAttack, Part::Attack], hostile, true))
}