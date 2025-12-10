use screeps::{RoomName, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::state::BoostReason;
use crate::rooms::state::requests::{Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::combat::fighter::Fighter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CaravanData {
    pub ambush_room: RoomName,
}

impl CaravanData {
    pub const fn new(ambush_room: RoomName) -> Self {
        Self { ambush_room }
    }
}

pub(in crate::rooms::state::requests) fn caravan_handler(
    data: &mut CaravanData,
    meta: &mut Meta,
    home_name: RoomName,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    if meta.created_at + 1500 > game::time() {
        match meta.status {
            Status::Created => {
                events.push(RoomEvent::AddBoost(BoostReason::Pvp, 1500));
                meta.update(Status::Spawning);
            }
            Status::Spawning if meta.updated_at + 25 < game::time() => {
                events.push(RoomEvent::Spawn(
                    Role::Fighter(Fighter::new(data.ambush_room, home_name, true)),
                    1,
                ));
                meta.update(Status::InProgress);
            }
            _ => {}
        }
    } else {
        meta.update(Status::Resolved);
    }
    events
}
