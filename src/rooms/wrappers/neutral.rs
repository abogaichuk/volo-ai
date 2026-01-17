use std::cmp;
use std::collections::BTreeMap;

use log::warn;
use screeps::{
    Creep, Deposit, EffectType, HasHits, HasId, HasPosition, OwnedStructureProperties,
    ResourceType, Room, RoomName, RoomObjectProperties, SYSTEM_USERNAME, SharedCreepProperties,
    StructureController, StructureInvaderCore, StructureObject, StructurePowerBank, Tombstone,
    find, game,
};

use crate::colony::ColonyEvent;
use crate::commons::{
    capture_room_numbers, find_walkable_positions_near_by, get_room_regex, is_highway,
    is_near_edge, is_skr,
};
use crate::utils::constants::DEPOSIT_REQUEST_THRESHOLD;

#[derive(Debug)]
pub struct Neutral {
    pub(crate) room_name: RoomName,
    pub(crate) controller: Option<StructureController>,
    pub(crate) icore: Option<StructureInvaderCore>,
    pub(crate) tombs: Vec<Tombstone>,
    pub(crate) deposits: Vec<Deposit>,
    pub(crate) power_banks: Vec<StructurePowerBank>,
    pub(crate) enemies: Vec<Creep>,
    pub(crate) is_blocked: bool,
}

impl Neutral {
    pub fn new(room: Room) -> Self {
        let tombs = room.find(find::TOMBSTONES, None);
        let enemies = room.find(find::HOSTILE_CREEPS, None);
        let deposits = room.find(find::DEPOSITS, None);
        let icore = room.find(find::HOSTILE_STRUCTURES, None).into_iter().find_map(|structure| {
            match structure {
                StructureObject::StructureInvaderCore(ic) => Some(ic),
                _ => None,
            }
        });
        let power_banks = room
            .find(find::STRUCTURES, None)
            .into_iter()
            .filter_map(|structure| match structure {
                StructureObject::StructurePowerBank(pb) => Some(pb),
                _ => None,
            })
            .collect();

        let is_blocked = room.find(find::STRUCTURES, None).into_iter().any(
            |structure| matches!(structure, StructureObject::StructureWall(w) if w.hits() == 0),
        );

        Self {
            room_name: room.name(),
            controller: room.controller(),
            icore,
            tombs,
            deposits,
            power_banks,
            enemies,
            is_blocked,
        }
    }

    pub fn run_room(&self) -> Vec<ColonyEvent> {
        let mut events = Vec::new();

        if self.is_blocked {
            return events;
        }

        if self.controller.as_ref().is_some_and(screeps::OwnedStructureProperties::my) {
            events.push(ColonyEvent::DeclareNew(self.room_name));
        } else {
            let re = get_room_regex();
            if let Some((f_num, s_num)) = capture_room_numbers(&re, self.room_name) {
                let (f_rem, s_rem) = (f_num % 10, s_num % 10);

                if is_skr(f_rem, s_rem) {
                    //not for farm, just check for IC existance
                    let ic_timeout = self
                        .icore
                        .as_ref()
                        .and_then(|ic| {
                            ic.effects().iter().find_map(|effect| {
                                match effect.effect() {
                                    //add 50 ticks to make sure a request with collapse timer has been created
                                    EffectType::NaturalEffect(_) => {
                                        Some(effect.ticks_remaining() + 50)
                                    }
                                    EffectType::PowerEffect(_) => None,
                                }
                            })
                        })
                        .unwrap_or_default();
                    if ic_timeout > 0 {
                        events.push(ColonyEvent::AvoidRoom(
                            self.room_name,
                            game::time() + ic_timeout,
                        ));
                    }
                } else if is_highway(f_rem, s_rem) {
                    events.extend(self.power_bank_events());
                    events.extend(self.deposit_events());
                    events.extend(self.withdraw_events());
                    events.extend(self.caravan_event());
                } else if let Some(controller) = self.controller.as_ref()
                    && controller.level() > 0
                {
                    //not for farm rooms with controller, ally, neutral, hostiles or reserved by
                    // somebody else
                    events.push(ColonyEvent::Expansion(
                        self.room_name,
                        controller.level(),
                        controller.owner().map(|owner| owner.username()),
                        controller.safe_mode().is_some_and(|mode| mode != 0),
                    ));
                } else {
                    //central square room?
                }
            } else {
                warn!("{} parts can't be captured!", self.room_name);
            }
        }
        events
    }

    fn withdraw_events(&self) -> impl Iterator<Item = ColonyEvent> + '_ {
        self.tombs.iter().filter_map(|tomb| {
            let amount = tomb.store().get_used_capacity(Some(ResourceType::CatalyzedGhodiumAcid));
            if amount > 500 {
                Some(ColonyEvent::Withdraw(
                    tomb.raw_id(),
                    tomb.pos(),
                    ResourceType::CatalyzedGhodiumAcid,
                    amount,
                ))
            } else {
                None
            }
        })
    }

    fn power_bank_events(&self) -> impl Iterator<Item = ColonyEvent> + '_ {
        self.power_banks.iter().filter_map(|pb| {
            if pb.ticks_to_decay() > 4000 && pb.power() >= 4500 && pb.hits() == 2_000_000 {
                Some(ColonyEvent::Powerbank(pb.id(), pb.pos(), pb.power()))
            } else {
                None
            }
        })
    }

    fn deposit_events(&self) -> impl Iterator<Item = ColonyEvent> + '_ {
        self.deposits.iter().filter_map(|deposit| {
            if deposit.last_cooldown() < DEPOSIT_REQUEST_THRESHOLD {
                Some(ColonyEvent::Deposit(
                    deposit.id(),
                    deposit.pos(),
                    cmp::min(3, find_walkable_positions_near_by(deposit.pos(), true).len()),
                ))
            } else {
                None
            }
        })
    }

    fn caravan_event(&self) -> Option<ColonyEvent> {
        let screeps: Vec<&Creep> = self
            .enemies
            .iter()
            .filter(|hostile| hostile.owner().username() == SYSTEM_USERNAME)
            .collect();

        if !screeps.is_empty()
            && screeps
                .iter()
                .all(|hostile| !hostile.pos().is_room_edge() && !is_near_edge(hostile.pos()))
        {
            let creeps: BTreeMap<String, u32> = screeps
                .into_iter()
                .map(|hostile| (hostile.name(), hostile.store().get_used_capacity(None)))
                .collect();

            if creeps.values().sum::<u32>() > 0 {
                Some(ColonyEvent::Caravan(creeps, self.room_name))
            } else {
                None
            }
        } else {
            None
        }
    }
}
