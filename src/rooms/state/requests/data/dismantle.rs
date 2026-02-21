use log::error;
use screeps::{ObjectId, Position, Structure, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::commons::is_walkable;
use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::BoostReason;
use crate::rooms::state::requests::assignment::Squad;
use crate::rooms::state::requests::{Assignment, Meta, Status};
use crate::units::roles::Role;
use crate::units::roles::services::dismantler::Dismantler;
use crate::units::roles::services::puller::Puller;
use crate::units::roles::teams::com_d::ComDismantler;
use crate::units::roles::teams::com_h::ComHealer;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DismantleData {
    pub id: ObjectId<Structure>,
    pub workplace: Position,
}

impl DismantleData {
    pub const fn new(id: ObjectId<Structure>, workplace: Position) -> Self {
        Self { id, workplace }
    }
}

pub(in crate::rooms::state::requests) fn dismantle_handler(
    data: &mut DismantleData,
    meta: &mut Meta,
    assignment: &mut Assignment,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    match meta.status {
        Status::Created => {
            match assignment {
                Assignment::Squads(squads) => {
                    let squad = Squad::new(data.id, squads.len() + 1);

                    let dismantler = Role::CombatDismantler(ComDismantler::new(Some(squad.id.clone()), Some(home.name())));
                    let healer = Role::CombatHealer(ComHealer::new(Some(squad.id.clone()), Some(home.name())));
                    
                    squads.push(squad);
                    events.push(RoomEvent::Spawn(dismantler, 1));
                    events.push(RoomEvent::Spawn(healer, 1));
                    events.push(RoomEvent::AddBoost(BoostReason::Dismantle, 300));

                    meta.update(Status::Spawning);
                }
                Assignment::Single(doer) => {
                    *doer = None;

                    let dismantler = Role::Dismantler(Dismantler::new(Some(home.name())));
                    let puller = Role::Puller(Puller::new(Some(home.name())));

                    events.push(RoomEvent::Spawn(dismantler, 1));
                    events.push(RoomEvent::Spawn(puller, 1));
                    events.push(RoomEvent::AddBoost(BoostReason::Dismantle, 300));

                    meta.update(Status::Spawning);
                }
                _ => {
                    error!("{:?} invalid assignment: {:?}", data, assignment);
                    meta.update(Status::Resolved);
                }
            }
        }
        Status::InProgress
            if game::time().is_multiple_of(100) && !assignment.has_alive_members() =>
        {
            meta.update(Status::Created);
        }
        Status::OnHold if is_walkable(data.workplace) => {
            meta.update(Status::Created);
        }
        _ => {}
    }
    events
}
