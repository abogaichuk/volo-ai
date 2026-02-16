use log::{info, warn};
use screeps::{Position, RoomName, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::combat::overseer::Overseer;
use crate::units::roles::haulers::hauler::Hauler;
use crate::units::roles::miners::sk_miner::SKMiner;
use crate::units::roles::services::house_keeper::HouseKeeper;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FarmData {
    pub room_name: RoomName,
}

impl FarmData {
    pub const fn new(room_name: RoomName) -> Self {
        Self { room_name }
    }
}

pub(in crate::rooms::state::requests) fn begin_farm_handler(
    data: &FarmData,
    meta: &mut Meta,
    _assignment: &mut Assignment,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    match meta.status {
        Status::Created => {
            //todo farm room could be unvisible in this tick
            if home.get_farm(data.room_name).and_then(|farm| farm.memory.plan()).is_some() {
                let overseer =
                    Role::Overseer(Overseer::new(Some(data.room_name), Some(home.name())));
                let house_keeper = Role::HouseKeeper(HouseKeeper::new(Some(home.name()), true));

                events.push(RoomEvent::Spawn(overseer, 1));
                events.push(RoomEvent::Spawn(house_keeper, 2));

                meta.update(Status::Spawning);
            } else {
                info!("{} can't begin farm: {}, plan does not exist!", home.name(), data.room_name);
                meta.update(Status::Created);
            }
        }
        Status::Spawning if meta.updated_at + 500 < game::time() => {
            if let Some(farm) = home.get_farm(data.room_name) {
                let containers = farm
                    .memory
                    .plan()
                    .map(|plan| plan.containers_near(&farm.sources))
                    .unwrap_or_default();

                if containers.is_empty() {
                    warn!(
                        "{} can't spawn miners, invalid plan for: {}",
                        home.name(),
                        data.room_name
                    );
                } else {
                    let spawn_events = containers.into_iter().map(|xy| {
                        let miner = Role::SkMiner(SKMiner::new(
                            Some(Position::new(xy.x, xy.y, data.room_name)),
                            Some(home.name()),
                        ));
                        RoomEvent::Spawn(miner, 1)
                    });
                    events.extend(spawn_events);
                    meta.update(Status::InProgress);
                }
            } else {
                warn!("{} invalid farm room: {}", home.name(), data.room_name);
            }
        }
        Status::InProgress if meta.updated_at + 1000 < game::time() => {
            let hauler = Role::Hauler(Hauler::new(Some(home.name()), true));
            events.push(RoomEvent::Spawn(hauler, 3));
            meta.update(Status::Resolved);
        }
        Status::OnHold if meta.updated_at + 100 < game::time() => {
            meta.update(Status::Created);
        }
        _ => {}
    }

    events
}
