use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use arrayvec::ArrayVec;
use enum_dispatch::enum_dispatch;
use screeps::{Creep, Part, ResourceType, RoomName};
use serde::{Deserialize, Serialize};

use self::combat::defender::Defender;
use self::combat::destroyer::Destroyer;
use self::combat::fighter::Fighter;
use self::combat::guard::Guard;
use self::combat::overseer::Overseer;
use self::haulers::carrier::Carrier;
use self::haulers::hauler::Hauler;
use self::miners::miner::Miner;
use self::miners::mineral_miner::MineralMiner;
use self::miners::sk_miner::SKMiner;
use self::services::booker::Booker;
use self::services::conqueror::Conqueror;
use self::services::dh::DismantlerWithHeal;
use self::services::dismantler::Dismantler;
use self::services::handyman::HandyMan;
use self::services::house_keeper::HouseKeeper;
use self::services::puller::Puller;
use self::services::remote_upgrader::RemoteUpgrader;
use self::services::scout::Scout;
use self::services::trader::Trader;
use self::services::upgrader::Upgrader;
use self::teams::com_d::ComDismantler;
use self::teams::com_h::ComHealer;
use self::teams::dep_hauler::DepositHauler;
use self::teams::dep_miner::DepositMiner;
use self::teams::pb_a::PBAttacker;
use self::teams::pb_c::PBCarrier;
use self::teams::pb_h::PBHealer;
use super::Task;
// use crate::creeps::{Home, roles::services::booker::Booker};
use crate::{movement::MovementProfile, rooms::shelter::Shelter};

pub mod combat;
pub mod haulers;
pub mod miners;
pub mod services;
pub mod teams;

#[enum_dispatch]
pub trait Kind {
    fn boosts(&self, _: &Creep) -> HashMap<Part, [ResourceType; 2]> {
        HashMap::new()
    }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile;

    fn get_task(&self, _: &Creep, _: &mut Shelter) -> Task {
        Task::Idle(10)
    }

    fn respawn_timeout(&self, _: Option<&Creep>) -> Option<usize> {
        None
    }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]>;
}

fn can_scale(
    mut body: ArrayVec<[Part; 50]>,
    body_extension: Vec<Part>,
    room_energy: u32,
    scale_limit: usize,
) -> bool {
    if body.len() + body_extension.len() > scale_limit {
        false
    } else {
        body.extend(body_extension);
        body.iter()
            .map(|part| part.cost())
            .reduce(|acc, e| acc + e)
            .is_some_and(|cost| cost < room_energy)
    }
}

const fn default_parts_priority(part: Part) -> i8 {
    match part {
        Part::Tough => 0,
        Part::Carry => 1,
        Part::Work | Part::Claim => 2,
        Part::Move => 3,
        Part::RangedAttack | Part::Attack => 4,
        Part::Heal => 10,
        _ => 5,
    }
}

const fn pvp_parts_priority(part: Part) -> i8 {
    match part {
        Part::Tough => 0,
        Part::Work | Part::Claim => 1,
        Part::Carry => 2,
        Part::RangedAttack | Part::Attack => 4,
        Part::Move => 7,
        Part::Heal => 10,
        _ => 5,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
#[enum_dispatch(Kind)]
pub enum Role {
    Upgrader(Upgrader),
    RemoteUpgrader(RemoteUpgrader),
    Miner(Miner),
    SkMiner(SKMiner),
    MineralMiner(MineralMiner),
    DepositMiner(DepositMiner),
    DepositHauler(DepositHauler),
    Hauler(Hauler),
    HandyMan(HandyMan),
    HouseKeeper(HouseKeeper),
    Scout(Scout),
    Conqueror(Conqueror),
    Booker(Booker),
    Overseer(Overseer),
    Defender(Defender),
    Trader(Trader),
    Dismantler(Dismantler),
    Guard(Guard),
    Carrier(Carrier),
    Puller(Puller),
    // ScoreCarrier,
    PBAttacker(PBAttacker),
    PBHealer(PBHealer),
    PBCarrier(PBCarrier),
    CombatDismantler(ComDismantler),
    CombatHealer(ComHealer),
    Destroyer(Destroyer),
    DismantlerWithHeal(DismantlerWithHeal),
    Fighter(Fighter),
}

impl Role {
    pub const fn set_home(&mut self, home: RoomName) {
        match self {
            Role::Upgrader(r) => r.home = Some(home),
            Role::RemoteUpgrader(r) => r.home = Some(home),
            Role::Miner(r) => r.home = Some(home),
            Role::MineralMiner(r) => r.home = Some(home),
            Role::SkMiner(r) => r.home = Some(home),
            Role::DepositMiner(r) => r.home = Some(home),
            Role::DepositHauler(r) => r.home = Some(home),
            Role::Hauler(r) => r.home = Some(home),
            Role::HandyMan(r) => r.home = Some(home),
            Role::HouseKeeper(r) => r.home = Some(home),
            Role::Scout(r) => r.home = Some(home),
            Role::Conqueror(r) => r.home = Some(home),
            Role::Booker(r) => r.home = Some(home),
            Role::Overseer(r) => r.home = Some(home),
            Role::Trader(r) => r.home = Some(home),
            Role::Dismantler(r) => r.home = Some(home),
            Role::Guard(r) => r.home = Some(home),
            Role::Carrier(r) => r.home = Some(home),
            Role::Puller(r) => r.home = Some(home),
            Role::Defender(r) => r.home = Some(home),
            Role::PBAttacker(r) => r.home = Some(home),
            Role::PBHealer(r) => r.home = Some(home),
            Role::PBCarrier(r) => r.home = Some(home),
            Role::CombatDismantler(r) => r.home = Some(home),
            Role::CombatHealer(r) => r.home = Some(home),
            Role::Destroyer(r) => r.home = Some(home),
            Role::DismantlerWithHeal(r) => r.home = Some(home),
            Role::Fighter(r) => r.home = Some(home),
        }
    }

    pub const fn get_home(&self) -> Option<&RoomName> {
        match self {
            Role::Upgrader(r) => r.home.as_ref(),
            Role::RemoteUpgrader(r) => r.home.as_ref(),
            Role::Miner(r) => r.home.as_ref(),
            Role::MineralMiner(r) => r.home.as_ref(),
            Role::SkMiner(r) => r.home.as_ref(),
            Role::DepositMiner(r) => r.home.as_ref(),
            Role::DepositHauler(r) => r.home.as_ref(),
            Role::Hauler(r) => r.home.as_ref(),
            Role::HandyMan(r) => r.home.as_ref(),
            Role::HouseKeeper(r) => r.home.as_ref(),
            Role::Scout(r) => r.home.as_ref(),
            Role::Conqueror(r) => r.home.as_ref(),
            Role::Booker(r) => r.home.as_ref(),
            Role::Overseer(r) => r.home.as_ref(),
            Role::Trader(r) => r.home.as_ref(),
            Role::Dismantler(r) => r.home.as_ref(),
            Role::Guard(r) => r.home.as_ref(),
            Role::Carrier(r) => r.home.as_ref(),
            Role::Puller(r) => r.home.as_ref(),
            Role::Defender(r) => r.home.as_ref(),
            Role::PBAttacker(r) => r.home.as_ref(),
            Role::PBHealer(r) => r.home.as_ref(),
            Role::PBCarrier(r) => r.home.as_ref(),
            // Role::Protector(r) => r.home,
            Role::CombatDismantler(r) => r.home.as_ref(),
            Role::CombatHealer(r) => r.home.as_ref(),
            Role::Destroyer(r) => r.home.as_ref(),
            Role::DismantlerWithHeal(r) => r.home.as_ref(),
            Role::Fighter(r) => r.home.as_ref(),
        }
    }

    //todo add invaded role priority
    /// The higher the more important
    pub const fn role_priority(&self) -> i8 {
        match self {
            Role::Guard(_) => 9,
            Role::Hauler(_) => 8,
            Role::Miner(_) => 7,
            Role::Defender(_) => 6,
            Role::Trader(_) => 5,
            Role::Overseer(_) | Role::Upgrader(_) | Role::PBAttacker(_) | Role::PBHealer(_) | Role::DepositMiner(_) => 4,
            Role::PBCarrier(_) | Role::DepositHauler(_) => 3,
            Role::SkMiner(_) | Role::Booker(_) | Role::HouseKeeper(_) => 2,
            Role::Carrier(_) | Role::Dismantler(_) | Role::Puller(_) => 1,
            Role::Scout(_) => -1,
            _ => 0,
        }
    }
}

impl Default for Role {
    fn default() -> Role {
        Role::Scout(Scout::new(None, None))
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = match self {
            Role::Upgrader(_) => "upgrader",
            Role::RemoteUpgrader(_) => "remote_upgrader",
            Role::Miner(_) => "miner",
            Role::MineralMiner(_) => "mineral_miner",
            Role::Hauler(_) => "hauler",
            Role::HandyMan(_) => "handyman",
            Role::HouseKeeper(_) => "house_keeper",
            Role::Scout(_) => "scout",
            Role::Conqueror(_) => "conqueror",
            Role::Booker(_) => "booker",
            Role::Defender(_) => "defender",
            Role::Trader(_) => "trader",
            Role::Dismantler(_) => "dismantler",
            Role::Guard(_) => "guard",
            Role::Carrier(_) => "carrier",
            Role::Puller(_) => "puller",
            // Role::ScoreCarrier => "score_carrier",
            Role::PBAttacker(_) => "pb_a",
            Role::PBHealer(_) => "pb_h",
            Role::PBCarrier(_) => "pb_c",
            Role::CombatDismantler(_) => "com_d",
            Role::CombatHealer(_) => "com_h",
            Role::Destroyer(_) => "destroyer",
            Role::DismantlerWithHeal(_) => "dh",
            Role::DepositMiner(_) => "dep_miner",
            Role::DepositHauler(_) => "dep_hauler",
            Role::Overseer(_) => "overseer",
            Role::SkMiner(_) => "sk_miner",
            Role::Fighter(_) => "fighter",
        };
        write!(f, "{name}")
    }
}
