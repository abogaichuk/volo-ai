use screeps::{HasHits, HasId, HasPosition, Position, RoomCoordinate, StructureRampart};
use crate::{
    rooms::{
        wrappers::claimed::Claimed,
        state::requests::{Request, RepairData, RequestKind, assignment::Assignment},
    },
    commons::get_positions_near_by,
    utils::constants::MAX_RAMPART_HITS
};

impl Claimed {
    pub(crate) fn run_ramparts(&self) -> impl Iterator<Item = Request> {
        let mut requests = Vec::new();

        let nuke_positions: Vec<Position> = self.nukes.iter()
            .flat_map(|nuke| get_positions_near_by(nuke.pos(), 3, false, true))
            .map(|(x, y)| Position::new(
                unsafe { RoomCoordinate::unchecked_new(x) },
                unsafe { RoomCoordinate::unchecked_new(y) },
                self.get_name()))
            .collect();
        
        let enemies = self.hostiles.len();
        for rampart in &self.ramparts {
            if enemies > 0 && rampart.is_public() {
                let _ = rampart.set_public(false);
            } else if enemies == 0 && !rampart.is_public() {
                let _ = rampart.set_public(true);
            }

            if need_repair(rampart, &nuke_positions) {
                requests.push(Request::new(
                    RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                        rampart.id().into_type(),
                        rampart.pos(),
                        25,
                        rampart.hits())),
                    Assignment::Single(None)));
            }
        }
        requests.into_iter()
    }
}

fn need_repair(rampart: &StructureRampart, nuke_positions: &[Position]) -> bool {
    rampart.hits() < MAX_RAMPART_HITS && (nuke_positions.is_empty() || nuke_positions.contains(&rampart.pos()))
}