use log::*;
use screeps::StructureStorage;
use std::collections::HashSet;
use crate::rooms::{
    RoomEvent, wrappers::claimed::Claimed,
    state::{RoomState, requests::{Request, RequestKind, meta::Status}},
};

//todo impl Shelter
impl Claimed {
    pub(crate) fn run_factory(&self, room_memory: &RoomState) -> Option<RoomEvent> {
        let Some(factory) = &self.factory else {
            return None;
        };

        debug!("{} running factory", self.get_name());
        let in_progress = room_memory.requests.iter()
            .any(|r| matches!(r.kind, RequestKind::Factory(_)) &&
                matches!(r.status(), Status::InProgress | Status::OnHold));

        (!in_progress)
            .then(|| {
                self.storage()
                    .and_then(|storage| {
                        if let Some(mut request) = new_request(&room_memory.requests, storage) {
                            request.join(None, None);
                            Some(RoomEvent::ReplaceRequest(request))
                        } else if room_memory.powers.contains(&screeps::PowerType::OperateFactory) {
                            Some(RoomEvent::DeletePower(screeps::PowerType::OperateFactory))
                        } else {
                            self.unload(factory, &[])
                        }
                    })
            })?
    }
}

fn new_request(requests: &HashSet<Request>, storage: &StructureStorage) -> Option<Request> {
    requests.iter()
        .find(|r| match &r.kind {
            RequestKind::Factory(d) => {
                d.resource.commodity_recipe()
                    .is_some_and(|recipe| recipe.components.iter()
                        .all(|(res, amount)| storage.store().get_used_capacity(Some(*res)) >= *amount))
            }
            _ => false
        })
        .cloned()
}