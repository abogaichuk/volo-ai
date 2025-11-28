use screeps::{HasHits, HasId, HasPosition, ObjectId, Position, RoomCoordinate, StructureRampart};
use crate::{
    commons::get_positions_near_by, rooms::{
        state::{constructions::{PlannedCell, RoomPlan, RoomStructure},
        requests::{RepairData, Request, RequestKind, assignment::Assignment}
    }, wrappers::claimed::Claimed
    }, utils::constants::MAX_RAMPART_HITS
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
        for rampart in self.ramparts.all() {
            rampart.toogle(enemies == 0);

            if rampart.need_repair(&nuke_positions) {
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

#[derive(Default)]
pub(crate) struct Ramparts {
    list: Vec<Rampart>
}

impl Ramparts {
    pub(crate) fn new(ramparts: Vec<StructureRampart>, plan: Option<&RoomPlan>) -> Self {
        let Some(plan) = plan else {
            return Self { 
                list: ramparts.into_iter()
                    .map(|r| Rampart { structure: r, perimeter: false })
                    .collect()
            };
        };

        let list = ramparts.into_iter()
            .map(|r| {
                let cell = PlannedCell::searchable(r.pos().xy(), RoomStructure::Rampart(false));
                let perimeter = plan.get_cell(cell)
                    .is_some_and(|planned| matches!(planned.structure, RoomStructure::Rampart(true)));
                Rampart { structure: r, perimeter }
            })
            .collect();
        Self { list }
    }

    fn all(&self) -> &[Rampart] {
        &self.list
    }

    pub(crate) fn perimeter(&self) -> impl Iterator<Item = &StructureRampart> {
        self.list.iter()
            .filter(|rampart| rampart.perimeter)
            .map(|rampart| &rampart.structure)
    }
}

pub(crate) struct Rampart {
    structure: StructureRampart,
    perimeter: bool
}

impl Rampart {
    fn toogle(&self, open: bool) {
        if self.perimeter {
            if open && !self.structure.is_public() {
                let _ = self.structure.set_public(true);
            } else if !open && self.structure.is_public() {
                let _ = self.structure.set_public(false);
            }
        } else if self.structure.is_public() {
            //permanently closed
            let _ = self.structure.set_public(false);
        }
    }

    fn need_repair(&self, nuke_positions: &[Position]) -> bool {
        self.structure.hits() < MAX_RAMPART_HITS && (nuke_positions.is_empty() || nuke_positions.contains(&self.structure.pos()))
    }

    fn hits(&self) -> u32 {
        self.structure.hits()
    }

    fn pos(&self) -> Position {
        self.structure.pos()
    }

    fn id(&self) -> ObjectId<StructureRampart> {
        self.structure.id()
    }
}