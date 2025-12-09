use screeps::game;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Meta {
    #[serde(default)]
    pub status: Status,
    #[serde(default = "game::time")]
    pub created_at: u32,
    #[serde(default = "game::time")]
    pub updated_at: u32,
}

impl Meta {
    pub fn update(&mut self, status: Status) {
        self.status = status;
        self.updated_at = game::time();
    }

    pub const fn is_finished(&self) -> bool {
        matches!(self.status, Status::Resolved | Status::Aborted)
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self { status: Status::Created, created_at: game::time(), updated_at: game::time() }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Status {
    OnHold,
    #[default]
    Created,
    InProgress,
    Spawning,
    Boosting,
    Carry,
    Aborted,
    Finishing,
    Resolved,
    Review,
}
