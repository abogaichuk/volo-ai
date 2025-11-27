use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};
use screeps::{RoomName, ResourceType, Part, Creep};
use std::{collections::HashMap, fmt::{self, Display, Formatter}};
// use crate::creeps::{Home, roles::services::booker::Booker};
use crate::{movement::MovementProfile, rooms::shelter::Shelter};
use super::{Task};
use enum_dispatch::enum_dispatch;
use self::{
    combat::{defender::Defender, destroyer::Destroyer, fighter::Fighter, guard::Guard, overseer::Overseer},
    haulers::{carrier::Carrier, hauler::Hauler},
    miners::{miner::Miner, mineral_miner::MineralMiner, sk_miner::SKMiner},
    services::{conqueror::Conqueror, booker::Booker, dh::DismantlerWithHeal, dismantler::Dismantler, handyman::HandyMan, puller::Puller,
        house_keeper::HouseKeeper, scout::Scout, trader::Trader, upgrader::Upgrader, remote_upgrader::RemoteUpgrader},
    teams::{com_d::ComDismantler, com_h::ComHealer, dep_hauler::DepositHauler, dep_miner::DepositMiner,
        pb_a::PBAttacker, pb_c::PBCarrier, pb_h::PBHealer}
};

pub mod combat;
pub mod haulers;
pub mod miners;
pub mod services;
pub mod teams;

#[enum_dispatch]
pub trait Kind {
    fn boosts(&self, _: &Creep) -> HashMap<Part, [ResourceType; 2]> { HashMap::new() }

    fn get_movement_profile(&self, creep: &Creep) -> MovementProfile;

    fn get_task(&self, _: &Creep, _: &mut Shelter) -> Task {
        Task::Idle(10)
    }

    fn respawn_timeout(&self, _: Option<&Creep>) -> Option<u32> { None }

    fn body(&self, room_energy: u32) -> ArrayVec<[Part; 50]>;
}

fn can_scale(mut body: ArrayVec<[Part; 50]>, body_extension: Vec<Part>, room_energy: u32, scale_limit: usize) -> bool {
    if body.len() + body_extension.len() > scale_limit {
        false
    } else {
        body.extend(body_extension);
        body
            .iter()
            .map(|part| part.cost())
            .reduce(|acc, e| acc + e)
            .is_some_and(|cost| cost < room_energy)
    }
}

fn default_parts_priority(part: Part) -> i8 {
    match part {
        Part::Tough => 0,
        Part::Carry => 1,
        Part::Work => 2,
        Part::Claim => 2,
        Part::Move => 3,
        Part::RangedAttack => 4,
        Part::Attack => 4,
        Part::Heal => 10,
        _ => 5
    }
}

fn pvp_parts_priority(part: Part) -> i8 {
    match part {
        Part::Tough => 0,
        Part::Work => 1,
        Part::Claim => 1,
        Part::Carry => 2,
        Part::RangedAttack => 4,
        Part::Attack => 4,
        Part::Move => 7,
        Part::Heal => 10,
        _ => 5
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
    Fighter(Fighter)
}

impl Role {
    pub fn set_home(&mut self, home: RoomName) {
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
            Role::Fighter(r) => r.home = Some(home)
        }
    }

    pub fn get_home(&self) -> Option<RoomName> {
        match self {
            Role::Upgrader(r) => r.home,
            Role::RemoteUpgrader(r) => r.home,
            Role::Miner(r) => r.home,
            Role::MineralMiner(r) => r.home,
            Role::SkMiner(r) => r.home,
            Role::DepositMiner(r) => r.home,
            Role::DepositHauler(r) => r.home,
            Role::Hauler(r) => r.home,
            Role::HandyMan(r) => r.home,
            Role::HouseKeeper(r) => r.home,
            Role::Scout(r) => r.home,
            Role::Conqueror(r) => r.home,
            Role::Booker(r) => r.home,
            Role::Overseer(r) => r.home,
            Role::Trader(r) => r.home,
            Role::Dismantler(r) => r.home,
            Role::Guard(r) => r.home,
            Role::Carrier(r) => r.home,
            Role::Puller(r) => r.home,
            Role::Defender(r) => r.home,
            Role::PBAttacker(r) => r.home,
            Role::PBHealer(r) => r.home,
            Role::PBCarrier(r) => r.home,
            // Role::Protector(r) => r.home,
            Role::CombatDismantler(r) => r.home,
            Role::CombatHealer(r) => r.home,
            Role::Destroyer(r) => r.home,
            Role::DismantlerWithHeal(r) => r.home,
            Role::Fighter(r) => r.home
        }
    }

    //todo add invaded role priority
    /// The higher the more important
    pub fn role_priority(&self) -> i8 {
        match self {
            Role::Guard(_) => 9,
            Role::Hauler(_) => 8,
            Role::Miner(_) => 7,
            Role::Defender(_) => 6,
            // Role::Hauler(_) => 6,
            Role::Trader(_) => 5,
            Role::Overseer(_) => 4,
            Role::Upgrader(_) => 4,
            Role::PBAttacker(_) => 4,
            Role::PBHealer(_) => 4,
            Role::DepositMiner(_) => 4,
            Role::PBCarrier(_) => 3,
            Role::DepositHauler(_) => 3,
            Role::SkMiner(_) => 2,
            Role::Booker(_) => 2,
            Role::HouseKeeper(_) => 2,
            Role::Carrier(_) => 1,
            Role::Dismantler(_) => 1,
            Role::Puller(_) => 1,
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
        write!(f, "{}", name)
    }
}