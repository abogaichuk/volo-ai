use serde::{Serialize, Deserialize};
use screeps::{game, RoomName};
use smallvec::SmallVec;
use crate::{
    rooms::{RoomEvent, state::{BoostReason, requests::{CreepHostile, Meta, Status}}},
    units::roles::{Role, combat::fighter::Fighter},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtectData {
    pub room_name: RoomName,
    #[serde(default)]
    pub hostiles: Vec<CreepHostile>,
    #[serde(default)]
    pub ctrl_level: u8,
}

impl ProtectData {
    pub fn new(room_name: RoomName, ctrl_level: u8) -> Self {
        Self { room_name, hostiles: Vec::new(), ctrl_level }
    }
}

pub(in crate::rooms::state::requests) fn protect_handler(
    data: &ProtectData,
    meta: &mut Meta,
    home_name: RoomName
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    if meta.created_at + 1650 > game::time() && meta.status == Status::Created {
        meta.update(Status::InProgress);

        match data.ctrl_level {
            1..=4 => {
                //low level ctrl, 1 tower max here
                events.push(RoomEvent::Spawn(Role::Fighter(Fighter::new(data.room_name, home_name, false)), 1));
            }
            _ => {
                //boost needed by default
                events.push(RoomEvent::AddBoost(BoostReason::Pvp, 750));
                events.push(RoomEvent::Spawn(Role::Fighter(Fighter::new(data.room_name, home_name, true)), 1));
            }
        }
    }
    events
}