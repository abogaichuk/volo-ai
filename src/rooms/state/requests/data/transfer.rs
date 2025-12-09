use std::str::FromStr;

use js_sys::JsString;
use log::{warn, info, error};
use screeps::{HasId, ResourceType, RoomName, game};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::requests::{Meta, Status};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferData {
    pub resource: ResourceType,
    pub amount: u32,
    pub destination: RoomName,
    #[serde(default)]
    pub description: Option<String>,
}

impl TransferData {
    pub const fn new(
        resource: ResourceType,
        amount: u32,
        destination: RoomName,
        description: Option<String>,
    ) -> Self {
        Self { resource, amount, destination, description }
    }
}

pub(in crate::rooms::state::requests) fn transfer_handler(
    data: &TransferData,
    meta: &mut Meta,
    home: &Shelter,
) -> SmallVec<[RoomEvent; 3]> {
    let mut events: SmallVec<[RoomEvent; 3]> = SmallVec::new();

    let Some(terminal) = home.terminal() else {
        return events;
    };

    match meta.status {
        Status::InProgress => {
            let transaction_cost = game::market::calc_transaction_cost(
                data.amount,
                &JsString::from_str(home.name().to_string().as_str())
                    .expect("couldn't resolve room_name"),
                &JsString::from_str(data.destination.to_string().as_str())
                    .expect("couldn't resolve room_name"),
            );

            //if transfer energy -> min energy capacity = self.amount + transaction_cost
            let transfer_amount = if data.resource == ResourceType::Energy {
                data.amount + transaction_cost
            } else {
                data.amount
            };

            let terminal_energy = terminal.store().get_used_capacity(Some(ResourceType::Energy));
            let terminal_resource = terminal.store().get_used_capacity(Some(data.resource));
            let free_capacity = terminal.store().get_free_capacity(None);

            if transaction_cost > terminal_energy {
                let lack = transaction_cost - terminal_energy;
                if free_capacity < lack as i32 {
                    if let Some(unload_event) = home.unload(terminal, &[data.resource]) {
                        events.push(unload_event);
                    } else {
                        warn!("{} terminal is full! request: {:?}", home.name(), data);
                    }
                } else if let Some(load_event) =
                    home.supply_resources(terminal.raw_id(), ResourceType::Energy, lack)
                {
                    meta.update(Status::OnHold);
                    events.push(load_event);
                } else {
                    warn!("{} not enough energy for request: {:?}", home.name(), data);
                }
            } else if transfer_amount > terminal_resource {
                let lack = transfer_amount - terminal_resource;
                if free_capacity < lack as i32 {
                    if let Some(unload_event) = home.unload(terminal, &[data.resource]) {
                        // events.push(RoomEvent::Request(RoomRequest::Transfer(self)));
                        events.push(unload_event);
                    } else {
                        warn!("{} terminal is full! request: {:?}", home.name(), data);
                    }
                } else if let Some(load_event) =
                    home.supply_resources(terminal.raw_id(), data.resource, lack)
                {
                    meta.update(Status::OnHold);
                    events.push(load_event);
                } else {
                    meta.update(Status::Aborted);
                    warn!("{} not enough resource for request: {:?}", home.name(), data);
                }
            } else if terminal.cooldown() == 0 {
                match terminal.send(
                    data.resource,
                    data.amount,
                    data.destination,
                    data.description.as_deref(),
                ) {
                    Ok(()) => info!(
                        "room: {}, send {}: {} to {} successfully!",
                        home.name(),
                        data.amount,
                        data.resource,
                        data.destination
                    ),
                    Err(err) => error!("transfer error: {:?}", err),
                }
                meta.update(Status::Resolved);
            }
        }
        Status::OnHold => {
            if meta.updated_at + 20 < game::time() {
                meta.update(Status::Created);
            }
        }
        _ => {}
    }
    events
}
