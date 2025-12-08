use std::cmp::min;

use screeps::{HasId, ResourceType};

use crate::commons::find_container_with;
use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::{CarryData, Request, RequestKind};
use crate::rooms::wrappers::claimed::Claimed;
use crate::utils::constants::MAX_CARRY_REQUEST_AMOUNT;

const GHODIUM_LOAD_CAPACITY: u32 = 5000;

impl Claimed {
    pub(crate) fn run_nuker(&self) -> Option<Request> {
        self.nuker.as_ref().and_then(|nuker| {
            if nuker.store().get_used_capacity(Some(ResourceType::Energy))
                < nuker.store().get_capacity(Some(ResourceType::Energy))
            {
                find_container_with(ResourceType::Energy, Some(150000), self.storage(), None, None)
                    .map(|(id, _)| {
                        Request::new(
                            RequestKind::Carry(CarryData::new(
                                id,
                                nuker.raw_id(),
                                ResourceType::Energy,
                                min(
                                    nuker.store().get_free_capacity(Some(ResourceType::Energy))
                                        as u32,
                                    MAX_CARRY_REQUEST_AMOUNT,
                                ),
                            )),
                            Assignment::Single(None),
                        )
                    })
            } else {
                let nuker_amount = nuker.store().get_used_capacity(Some(ResourceType::Ghodium));
                if nuker_amount < GHODIUM_LOAD_CAPACITY {
                    if let Some(storage) = self.storage() {
                        let storage_amount =
                            storage.store().get_used_capacity(Some(ResourceType::Ghodium));
                        if storage_amount > 0 {
                            return Some(Request::new(
                                RequestKind::Carry(CarryData::new(
                                    storage.raw_id(),
                                    nuker.raw_id(),
                                    ResourceType::Ghodium,
                                    min(GHODIUM_LOAD_CAPACITY - nuker_amount, storage_amount),
                                )),
                                Assignment::Single(None),
                            ));
                        }
                    }
                }
                None
            }
        })
    }
}
