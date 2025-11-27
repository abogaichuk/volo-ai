use serde::{Serialize, Deserialize};
use screeps::{game, RoomName};
use smallvec::SmallVec;
use crate::{
    units::roles::{Role, combat::fighter::Fighter},
    rooms::{state::BoostReason, RoomEvent, state::requests::{Meta, Status}}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CaravanData {
    pub ambush_room: RoomName
}

impl CaravanData {
    pub fn new(ambush_room: RoomName) -> Self {
        Self { ambush_room }
    }
}

pub(in crate::rooms::state::requests) fn caravan_handler(
    data: &mut CaravanData,
    meta: &mut Meta,
    home_name: RoomName
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    if meta.created_at + 1500 > game::time() {
        match meta.status {
            Status::Created => {
                events.push(RoomEvent::AddBoost(BoostReason::Pvp, 1500));
                meta.update(Status::Spawning);
            }
            Status::Spawning if meta.updated_at + 25 < game::time() => {
                events.push(RoomEvent::Spawn(Role::Fighter(Fighter::new(data.ambush_room, home_name, true)), 1));
                meta.update(Status::InProgress);
            }
            _ => {}
        }
    } else {
        meta.update(Status::Resolved);
    }
    events
}