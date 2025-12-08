use log::*;
use screeps::action_error_codes::SpawnCreepErrorCode;
use screeps::{ResourceType, SpawnOptions, StructureProperties, StructureSpawn, game};
use smallvec::SmallVec;

use crate::rooms::RoomEvent;
use crate::rooms::state::RoomState;
use crate::rooms::wrappers::claimed::Claimed;
use crate::units::roles::{Kind, Role};

const MIN_ENERGY_AMOUNT: u32 = 5000;

impl Claimed {
    //todo bug, miner spawned with only 1 carry part = no work and no move
    pub(crate) fn run_spawns(&self, room_memory: &RoomState) -> SmallVec<[RoomEvent; 2]> {
        let mut events: SmallVec<[RoomEvent; 2]> = SmallVec::new();

        if !room_memory.spawns.is_empty() {
            if room_memory.spawns.len() > 1
                && !room_memory.powers.contains(&screeps::PowerType::OperateSpawn)
            {
                events.push(RoomEvent::AddPower(screeps::PowerType::OperateSpawn));
            }
            events.extend(self.try_spawn(room_memory));
        } else if room_memory.powers.contains(&screeps::PowerType::OperateSpawn) {
            events.push(RoomEvent::DeletePower(screeps::PowerType::OperateSpawn));
        }
        events
    }

    fn try_spawn(&self, room_memory: &RoomState) -> Option<RoomEvent> {
        get_spawn_data_by_priority(&room_memory.spawns).and_then(|(index, spawn_role)| {
            if let Some(spawn) = self.find_available_spawn() {
                let max_scale = self
                    .storage()
                    .map(|storage| {
                        storage.store().get_used_capacity(Some(ResourceType::Energy))
                            > MIN_ENERGY_AMOUNT
                            && !room_memory.origin
                    })
                    .unwrap_or(!room_memory.origin);

                let room_energy = if max_scale {
                    self.energy_capacity_available()
                } else {
                    self.energy_available()
                };

                if !max_scale {
                    warn!(
                        "{} spawn creeps with limited scales! room_energy {}",
                        self.get_name(),
                        room_energy
                    );
                }

                let body = spawn_role.body(room_energy);
                let spawn_options = SpawnOptions::new().dry_run(true);
                match spawn.spawn_creep_with_options(&body, "___test_name", &spawn_options) {
                    Ok(_) => {
                        let js_memory = serde_wasm_bindgen::to_value(&spawn_role).unwrap();
                        let spawn_options = SpawnOptions::new().memory(js_memory);

                        let mut sufix: u32 = 0;
                        'spawn_loop: loop {
                            let name = create_name(spawn_role, sufix);
                            match spawn.spawn_creep_with_options(&body, &name, &spawn_options) {
                                Ok(_) => {
                                    debug!(
                                        "room: {} successfully spawned creep: {}",
                                        self.get_name(),
                                        name
                                    );
                                    break 'spawn_loop Some(RoomEvent::Spawned(
                                        name,
                                        spawn_role.clone(),
                                        index,
                                    ));
                                }
                                Err(err) => match err {
                                    SpawnCreepErrorCode::NameExists => {
                                        sufix += 1;
                                    }
                                    _ => {
                                        break 'spawn_loop None;
                                    }
                                },
                            }
                        }
                    }
                    Err(err) => {
                        let message = format!(
                            "{} can't spawn role: {:?}, error: {:?}",
                            self.get_name(),
                            spawn_role,
                            err
                        );
                        debug!("{}", message);
                        None
                    }
                }
            } else {
                None
            }
        })
    }

    fn find_available_spawn(&self) -> Option<&StructureSpawn> {
        self.spawns.iter().find(|s| s.spawning().is_none() && s.is_active())
    }
}

fn create_name(role: &Role, prefix: u32) -> String {
    let time = game::time() % 10_000;
    format!("{}_{:04x}", role, time + prefix)
}

fn get_spawn_data_by_priority(spawns: &[Role]) -> Option<(usize, &Role)> {
    spawns.iter().enumerate().fold(
        None,
        |result: Option<(usize, &Role)>, (index, spawn_role)| -> Option<(usize, &Role)> {
            result
                .map(|result| {
                    let prev_result = result.1.role_priority();
                    let cur_result = spawn_role.role_priority();

                    if prev_result >= cur_result { result } else { (index, spawn_role) }
                })
                .or(Some((index, spawn_role)))
        },
    )
}
