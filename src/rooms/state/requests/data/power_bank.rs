use log::*;
use serde::{Serialize, Deserialize};
use screeps::{game, ObjectId, StructurePowerBank, ResourceType, Position, Part, HasHits};
use smallvec::SmallVec;
use crate::{
    rooms::{RoomEvent, shelter::Shelter, state::{requests::{Assignment, Meta, Status}}},
    units::roles::{Role, teams::{pb_a::PBAttacker, pb_h::PBHealer, pb_c::PBCarrier}},
    utils::{commons::find_hostiles_nearby, constants::MAX_POWER_CAPACITY}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PowerbankData {
    pub id: ObjectId<StructurePowerBank>,
    pub pos: Position,
    pub amount: u32,
    #[serde(default)]
    pub postponed_farm: bool
}

impl PowerbankData {
    pub fn new(id: ObjectId<StructurePowerBank>, pos: Position, amount: u32) -> Self {
        Self { id, pos, amount, postponed_farm: false }
    }

    pub fn postponed(id: ObjectId<StructurePowerBank>, pos: Position, amount: u32) -> Self {
        Self { id, pos, amount, postponed_farm: true }
    }
}

pub(in crate::rooms::state::requests) fn powerbank_handler(
    data: &PowerbankData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home: &Shelter
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    let time = game::time();

    if meta.created_at + 5200 > time {
        match meta.status {
            Status::Created if home.storage()
                .is_some_and(|storage| storage.store().get_used_capacity(Some(ResourceType::Power)) > MAX_POWER_CAPACITY) =>
            {
                meta.update(Status::Aborted);
            }
            Status::Created if data.postponed_farm => {
                if let Some(power_bank) = data.id.resolve() {
                    if power_bank.hits() == power_bank.hits_max() && power_bank.ticks_to_decay() < 4300 {
                        if let Some(squad_id) = assignment.new_squad(data.id.to_string(), meta) {
                            meta.update(Status::InProgress);

                            let pb_a = Role::PBAttacker(PBAttacker::new(Some(squad_id.clone()), Some(home.name())));
                            let pb_h = Role::PBHealer(PBHealer::new(Some(squad_id), Some(home.name())));

                            events.push(RoomEvent::Spawn(pb_h, 1));
                            events.push(RoomEvent::Spawn(pb_a, 1));
                        } else {
                            warn!("creation new squad error: {:?}", data);
                        }
                    } else if power_bank.hits() < power_bank.hits_max() {
                        meta.update(Status::Aborted);
                    }
                }
            }
            Status::Created => {
                if let Some(squad_id) = assignment.new_squad(data.id.to_string(), meta) {
                    meta.update(Status::InProgress);

                    let pb_a = Role::PBAttacker(PBAttacker::new(Some(squad_id.clone()), Some(home.name())));
                    let pb_h = Role::PBHealer(PBHealer::new(Some(squad_id), Some(home.name())));

                    events.push(RoomEvent::Spawn(pb_h, 1));
                    events.push(RoomEvent::Spawn(pb_a, 1));
                } else {
                    warn!("creation new squad error: {:?}", data);
                }
            },
            Status::InProgress if meta.updated_at + 1350 < time => {
                if let Some(squad_id) = assignment.new_squad(data.id.to_string(), meta) {
                    meta.update(Status::InProgress);

                    let pb_a = Role::PBAttacker(PBAttacker::new(Some(squad_id.clone()), Some(home.name())));
                    let pb_h = Role::PBHealer(PBHealer::new(Some(squad_id), Some(home.name())));

                    events.push(RoomEvent::Spawn(pb_h, 1));
                    events.push(RoomEvent::Spawn(pb_a, 1));
                } else {
                    warn!("creation new squad error: {:?}", data);
                }
            }
            Status::InProgress => {
                if let Some(power_bank) = data.id.resolve() {
                    //todo calculate distance
                    let pb_room = power_bank.room()
                        .expect("expect powerbank is in a room");
                    let another_attacker = find_hostiles_nearby(&pb_room, vec![Part::Attack], &power_bank).count();

                    if (power_bank.hits() < 600000 && another_attacker > 0) || power_bank.hits() < 400000 {
                        if let Some(squad_id) = assignment.new_squad(data.id.to_string(), meta) {
                            meta.update(Status::Carry);

                            let pb_c = Role::PBCarrier(PBCarrier::new(Some(squad_id.clone()), Some(home.name())));
                            events.push(RoomEvent::Spawn(pb_c, (data.amount as f32 / 1600_f32).round() as usize));
                        } else {
                            warn!("creation new squad error: {:?}", data);
                        }
                    }
                }
            },
            _ => {}
        }
    }
    events
}