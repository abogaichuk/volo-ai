use screeps::{HasHits, HasId, HasPosition, ResourceType};

use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::{CarryData, RepairData, Request, RequestKind, WithdrawData};
use crate::rooms::wrappers::claimed::Claimed;

impl Claimed {
    pub(crate) fn run_containers(&self) -> impl Iterator<Item = Request> {
        let mut requests = Vec::new();
        for container in &self.containers {
            if self.controller.pos().get_range_to(container.pos()) <= 2 {
                if let Some(storage) = &self.storage
                    && storage.store().get_used_capacity(Some(ResourceType::Energy)) > 10000
                    && container.store().get_used_capacity(Some(ResourceType::Energy)) <= 1200
                {
                    requests.push(Request::new(
                        RequestKind::Carry(CarryData::new(
                            storage.raw_id(),
                            container.raw_id(),
                            ResourceType::Energy,
                            800,
                        )),
                        Assignment::Single(None),
                    ));
                }
            } else if container.store().get_used_capacity(None) >= 1400 {
                requests.push(Request::new(
                    RequestKind::Withdraw(WithdrawData::new(
                        container.id().into(),
                        container.pos(),
                        container
                            .store()
                            .store_types()
                            .into_iter()
                            .map(|res| (res, None))
                            .collect(),
                    )),
                    Assignment::Single(None),
                ));
            }

            if container.hits() < ((container.hits_max() as f32 * 0.75) as u32) {
                requests.push(Request::new(
                    RequestKind::Repair(RepairData::with_max_attempts_and_hits(
                        container.id().into_type(),
                        container.pos(),
                        10,
                        container.hits(),
                    )),
                    Assignment::Single(None),
                ));
            }
        }
        requests.into_iter()
    }
}
