use log::*;
use std::collections::HashSet;
use crate::rooms::{
    RoomEvent, wrappers::claimed::Claimed,
    state::{RoomState, requests::{Request, RequestKind, meta::Status}},
};

impl Claimed {
    pub(crate) fn run_factory(&self, requests: &HashSet<Request>, room_memory: &RoomState) -> Option<RoomEvent> {
        let Some(factory) = &self.factory else {
            return None;
        };

        debug!("{} running factory", self.get_name());
        let in_progress = requests.iter()
            .any(|r| matches!(r.kind, RequestKind::Factory(_)) &&
                matches!(r.status(), Status::InProgress | Status::OnHold));

        (!in_progress)
            .then(|| {
                if let Some(mut request) = new_request(requests) {
                    request.join(None, None);
                    Some(RoomEvent::ReplaceRequest(request))
                } else {
                    self.unload(factory, &[])
                }
            })?
    }
}

fn new_request(requests: &HashSet<Request>) -> Option<Request> {
    requests.iter()
        .find(|r| matches!(r.kind, RequestKind::Factory(_)) &&
            matches!(r.status(), Status::Created))
        .cloned()
}