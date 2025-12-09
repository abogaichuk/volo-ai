use std::cmp::min;

use log::warn;
use screeps::{Deposit, HasPosition, ObjectId, Part, Position, RoomName, find, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::teams::dep_hauler::DepositHauler;
use crate::units::roles::teams::dep_miner::DepositMiner;
use crate::utils::commons::has_part;
use crate::utils::constants::DEPOSIT_REQUEST_THRESHOLD;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositData {
    pub id: ObjectId<Deposit>,
    pub pos: Position,
    pub empty_cells: usize,
}

impl DepositData {
    pub const fn new(id: ObjectId<Deposit>, pos: Position, empty_cells: usize) -> Self {
        Self { id, pos, empty_cells }
    }
}

pub(in crate::rooms::state::requests) fn deposit_handler(
    data: &mut DepositData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home_name: RoomName,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();
    match meta.status {
        Status::Created => {
            if let Some(squad_id) = assignment.new_squad(data.id.to_string(), meta) {
                let dep_miner =
                    Role::DepositMiner(DepositMiner::new(Some(squad_id.clone()), Some(home_name)));
                let dep_hauler = Role::DepositHauler(DepositHauler::new(
                    Some(squad_id),
                    Some(home_name),
                ));

                events.push(RoomEvent::Spawn(dep_miner, min(3, data.empty_cells)));
                events.push(RoomEvent::Spawn(dep_hauler, 1));
            } else {
                warn!("creation new squad error: {:?}", data);
            }
        }
        Status::InProgress => {
            if let Some(deposit) = data.id.resolve()
                && deposit.last_cooldown() >= DEPOSIT_REQUEST_THRESHOLD
            {
                meta.update(Status::Carry);
            } else if game::time() > meta.updated_at + 1350 {
                let fast_spawn = game::rooms()
                    .get(data.pos.room_name())
                    .is_some_and(|room| {
                        room.find(find::HOSTILE_CREEPS, None).iter().any(|hostile| {
                            has_part(&[Part::Work], hostile, false)
                                && hostile.pos().in_range_to(data.pos, 5)
                        })
                    });

                if fast_spawn || game::time() > meta.updated_at + 1400 {
                    if let Some(squad_id) = assignment.new_squad(data.id.to_string(), meta) {
                        let dep_miner = Role::DepositMiner(DepositMiner::new(
                            Some(squad_id.clone()),
                            Some(home_name),
                        ));
                        let dep_hauler = Role::DepositHauler(DepositHauler::new(
                            Some(squad_id),
                            Some(home_name),
                        ));

                        events.push(RoomEvent::Spawn(dep_miner, min(3, data.empty_cells)));
                        events.push(RoomEvent::Spawn(dep_hauler, 1));
                    } else {
                        warn!("creation new squad error: {:?}", data);
                    }
                }
            }
        }
        Status::Carry if meta.updated_at < game::time() - 2000 => {
            meta.update(Status::Resolved);
        }
        _ => {}
    }
    events
}
