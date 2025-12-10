use std::hash::{Hash, Hasher};

use screeps::{ResourceType, RoomName, game};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ResourceOrder {
    pub(crate) from: RoomName,
    pub(crate) to: Option<RoomName>,
    pub(crate) resource: ResourceType,
    pub(crate) amount: u32,
    pub(crate) timeout: u32,
}

impl Eq for ResourceOrder {}
impl PartialEq for ResourceOrder {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.resource == other.resource
    }
}

impl Hash for ResourceOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.from.hash(state);
        self.resource.hash(state);
    }
}

impl ResourceOrder {
    pub(crate) fn new(from: RoomName, resource: ResourceType, amount: u32) -> Self {
        Self { from, to: None, resource, amount, timeout: game::time() + 100 }
    }
}
