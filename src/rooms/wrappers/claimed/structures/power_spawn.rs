use log::error;
use screeps::action_error_codes::ProcessPowerErrorCode;
use screeps::{HasId, ResourceType};

use crate::commons::find_container_with;
use crate::rooms::state::requests::assignment::Assignment;
use crate::rooms::state::requests::{CarryData, Request, RequestKind};
use crate::rooms::wrappers::claimed::Claimed;
const POWER_LOAD_CAPACITY: u32 = 100;
const MIN_ENERGY_AMOUNT: u32 = 250000;

impl Claimed {
    pub(crate) fn run_power(&self) -> Option<Request> {
        self.power_spawn.as_ref().and_then(|power_spawn| match power_spawn.process_power() {
            Ok(()) => None,
            Err(err) => if err == ProcessPowerErrorCode::NotEnoughResources {
                if power_spawn.store().get_used_capacity(Some(ResourceType::Power)) == 0 {
                    find_container_with(
                        ResourceType::Power,
                        Some(POWER_LOAD_CAPACITY),
                        self.storage(),
                        None,
                        None,
                    )
                    .map(|(id, _)| {
                        Request::new(
                            RequestKind::Carry(CarryData::new(
                                id,
                                power_spawn.raw_id(),
                                ResourceType::Power,
                                POWER_LOAD_CAPACITY,
                            )),
                            Assignment::Single(None),
                        )
                    })
                } else if power_spawn.store().get_used_capacity(Some(ResourceType::Energy)) < 50
                {
                    find_container_with(
                        ResourceType::Energy,
                        Some(MIN_ENERGY_AMOUNT),
                        self.storage(),
                        None,
                        None,
                    )
                    .map(|(id, _)| {
                        Request::new(
                            RequestKind::Carry(CarryData::new(
                                id,
                                power_spawn.raw_id(),
                                ResourceType::Energy,
                                4000,
                            )),
                            Assignment::Single(None),
                        )
                    })
                } else {
                    None
                }
            } else {
                error!("process_power error: {:?}", err);
                None
            },
        })
    }
}
