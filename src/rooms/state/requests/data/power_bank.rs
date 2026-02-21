use log::warn;
use screeps::{HasHits, ObjectId, Part, Position, ResourceType, RoomName, StructurePowerBank, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::assignment::Squad;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::teams::pb_a::PBAttacker;
use crate::units::roles::teams::pb_c::PBCarrier;
use crate::units::roles::teams::pb_h::PBHealer;
use crate::utils::commons::find_hostiles_nearby;
use crate::utils::constants::MAX_POWER_CAPACITY;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PowerbankData {
    pub id: ObjectId<StructurePowerBank>,
    pub pos: Position,
    pub amount: u32,
    #[serde(default)]
    pub postponed_farm: bool,
}

impl PowerbankData {
    pub const fn new(id: ObjectId<StructurePowerBank>, pos: Position, amount: u32) -> Self {
        Self { id, pos, amount, postponed_farm: false }
    }

    pub const fn postponed(id: ObjectId<StructurePowerBank>, pos: Position, amount: u32) -> Self {
        Self { id, pos, amount, postponed_farm: true }
    }

    fn spawn_attack_squad(
        &self,
        meta: &mut Meta,
        assignment: &mut Assignment,
        home_name: RoomName,
        events: &mut SmallVec<[RoomEvent; 3]>)
    {
        if let Assignment::Squads(squads) = assignment {
            let squad = Squad::new(self.id, squads.len() + 1);

            let pb_a = Role::PBAttacker(PBAttacker::new(
                Some(squad.id.clone()),
                Some(home_name),
            ));
            let pb_h =
                Role::PBHealer(PBHealer::new(Some(squad.id.clone()), Some(home_name)));

            squads.push(squad);
            events.push(RoomEvent::Spawn(pb_h, 1));
            events.push(RoomEvent::Spawn(pb_a, 1));

            meta.update(Status::InProgress);
        } else {
            warn!("creation new squad error: {:?}", self);
        }
    }

    fn spawn_carry_squad(
        &self,
        meta: &mut Meta,
        assignment: &mut Assignment,
        home_name: RoomName,
        events: &mut SmallVec<[RoomEvent; 3]>)
    {
        if let Assignment::Squads(squads) = assignment {
            let squad = Squad::new(self.id, squads.len() + 1);

            let pb_c =
                Role::PBCarrier(PBCarrier::new(Some(squad.id.clone()), Some(home_name)));

            squads.push(squad);
            events.push(RoomEvent::Spawn(
                pb_c,
                ((self.amount + 800) / 1600) as usize, //(data.amount as f32 / 1600_f32).round() as usize,
            ));

            meta.update(Status::Carry);
        } else {
            warn!("creation new squad error: {:?}", self);
        }
    }

}

pub(in crate::rooms::state::requests) fn powerbank_handler(
    data: &PowerbankData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    if meta.created_at + 5200 > game::time() {
        match meta.status {
            Status::Created
                if home.storage().is_some_and(|storage| {
                    storage.store().get_used_capacity(Some(ResourceType::Power))
                        > MAX_POWER_CAPACITY
                }) =>
            {
                meta.update(Status::Aborted);
            }
            Status::Created if data.postponed_farm => {
                if let Some(power_bank) = data.id.resolve() {
                    if power_bank.hits() == power_bank.hits_max()
                        && power_bank.ticks_to_decay() < 4300
                    {
                        data.spawn_attack_squad(meta, assignment, home.name(), &mut events);
                    } else if power_bank.hits() < power_bank.hits_max() {
                        meta.update(Status::Aborted);
                    }
                }
            }
            Status::Created => {
                data.spawn_attack_squad(meta, assignment, home.name(), &mut events);
            }
            Status::InProgress if meta.updated_at + 1350 < game::time() => {
                data.spawn_attack_squad(meta, assignment, home.name(), &mut events);
            }
            Status::InProgress => {
                if let Some(power_bank) = data.id.resolve() {
                    //todo calculate distance
                    let pb_room = power_bank.room().expect("expect powerbank is in a room");
                    let another_attacker =
                        find_hostiles_nearby(&pb_room, vec![Part::Attack], &power_bank).count();

                    if (power_bank.hits() < 600_000 && another_attacker > 0)
                        || power_bank.hits() < 400_000
                    {
                        data.spawn_carry_squad(meta, assignment, home.name(), &mut events);
                    }
                }
            }
            _ => {}
        }
    } else {
        meta.update(Status::Resolved);
    }
    events
}
