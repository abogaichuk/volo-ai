use log::{debug, warn};
use screeps::{
    Creep, HasPosition, ObjectId, RawObjectId, ResourceType, SharedCreepProperties,
    StructureContainer, StructureController, game,
};
use wasm_bindgen::JsCast;

use crate::movement::walker::Walker;
use crate::units::roles::Role;
use crate::units::{Task, TaskResult};
use crate::utils::constants::LONG_RANGE_ACTION;

pub fn upgrade(
    id: ObjectId<StructureController>,
    container_id: Option<RawObjectId>,
    creep: &Creep,
    role: &Role,
    enemies: Vec<Creep>,
) -> TaskResult {
    if let Some(controller) = id.resolve() {
        if creep.pos().in_range_to(controller.pos(), LONG_RANGE_ACTION)
            && creep.pos().room_name() == controller.pos().room_name()
        {
            match creep.upgrade_controller(&controller) {
                Ok(()) => {
                    let _ = creep.say("⬆", false);
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) < 30
                        && let Some(room_obj) =
                            container_id.and_then(|c_id| game::get_object_by_id_erased(&c_id))
                    {
                        let container = room_obj.unchecked_ref::<StructureContainer>();
                        let _ = creep.withdraw(container, ResourceType::Energy, None);
                    }
                    TaskResult::StillWorking(Task::Upgrade(id, container_id), None)
                }
                Err(err) => {
                    debug!(
                        "creep: {} can't upgrade controller: {}, error: {:?}",
                        creep.name(),
                        id,
                        err
                    );
                    TaskResult::Abort
                }
            }
        } else {
            let goal =
                Walker::Reinforcing.walk(controller.pos(), LONG_RANGE_ACTION, creep, role, enemies);
            TaskResult::StillWorking(Task::Upgrade(id, container_id), Some(goal))
        }
    } else {
        warn!("{} controller not found! {}", creep.name(), id);
        TaskResult::Abort
    }
}
