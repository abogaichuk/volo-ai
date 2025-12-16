use log::debug;

use crate::rooms::{
    RoomEvent,
    shelter::Shelter,
    state::requests::{Request, RequestKind, meta::Status},
};

impl Shelter<'_> {
    pub(crate) fn run_factory(&self) -> Option<RoomEvent> {
        let factory = self.base.factory()?;

        debug!("{} running factory", self.name());
        let in_progress = self.requests().any(|r| {
            matches!(r.kind, RequestKind::Factory(_))
                && matches!(r.status(), Status::InProgress | Status::OnHold)
        });

        (!in_progress).then(|| {
            if let Some(mut request) = self.get_factory_request() {
                request.join(None, None);
                Some(RoomEvent::ReplaceRequest(request))
            } else if self.is_power_enabled(screeps::PowerType::OperateFactory) {
                Some(RoomEvent::DeletePower(screeps::PowerType::OperateFactory))
            } else {
                self.unload(factory, &[])
            }
        })?
    }

    fn get_factory_request(&self) -> Option<Request> {
        let storage = self.base.storage()?;

        self.requests()
            .find(|r| match &r.kind {
                RequestKind::Factory(d) => d.resource.commodity_recipe().is_some_and(|recipe| {
                    recipe.components.iter().all(|(res, amount)| {
                        storage.store().get_used_capacity(Some(*res)) >= *amount
                    })
                }),
                _ => false,
            })
            .cloned()
    }
}
