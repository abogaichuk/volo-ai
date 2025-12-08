use std::str::FromStr;

use log::*;
use ordered_float::OrderedFloat;
use screeps::{
    OrderType, OwnedStructureProperties, ResourceType, RoomName, RoomXY, StructureObject,
    StructureProperties, find, game,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::GLOBAL_MEMORY;
use crate::rooms::state::constructions::{PlannedCell, RoomStructure};
use crate::rooms::state::requests::Request;
use crate::rooms::state::{BoostReason, FarmInfo, RoomState, TradeData};
use crate::units::creeps::CreepMemory;
use crate::units::roles::Role;

#[wasm_bindgen]
pub fn info() -> String {
    GLOBAL_MEMORY.with(|mem_refcell| {
        let state = mem_refcell.borrow();
        let rooms_info = state.rooms.iter().fold("".to_string(), |acc, elem| {
            let room_header = format!("{}:\n", elem.0);

            let spawns = elem
                .1
                .spawns
                .iter()
                .map(|spawn| format!("{:?}\n", spawn))
                .reduce(|acc, line| format!("{}{}", acc, line))
                .unwrap_or_else(|| "[]".to_string());
            let spawn_info = format!("     spawns: [{}\n]\n", spawns);

            let requests_info = format!("     requests size: {}\n", elem.1.requests.len());

            // let min_hits =  elem.1.perimetr.iter()
            //     .map(|id| id.resolve()
            //         .map_or_else(||0, |rampart| rampart.hits()))
            //     .min().unwrap_or(0);
            // let perimetr_info = format!("     min perimeter hits: {}\n", min_hits);

            // format!("{}{}{}{}{}", acc, room_header, spawn_info, requests_info,
            // perimetr_info)
            format!("{}{}{}{}", acc, room_header, spawn_info, requests_info)
        });
        rooms_info
    })
}

#[wasm_bindgen]
pub fn c_info() -> usize {
    game::creeps().entries().count()
}

#[wasm_bindgen]
pub fn spawn(room_name: String, creep: JsValue) -> String {
    match serde_wasm_bindgen::from_value::<Role>(creep) {
        Ok(mut role) => match RoomName::from_str(&room_name) {
            Ok(room_name) => {
                role.set_home(room_name);
                GLOBAL_MEMORY.with(|mem_refcell| {
                    match mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                        Some(claimed) => {
                            let message = format!("room_name: {}, added new {}", room_name, role);
                            claimed.add_to_spawn(role, 1);
                            message
                        }
                        _ => {
                            format!("room: {} is not claimed room", room_name)
                        }
                    }
                })
            }
            Err(error) => {
                format!("incorrect room name: {}", error)
            }
        },
        Err(error) => {
            format!("incorrect creep data: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn request(room_name: String, request_js: JsValue) -> String {
    match serde_wasm_bindgen::from_value::<Request>(request_js.clone()) {
        Ok(request) => match RoomName::from_str(&room_name) {
            Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
                match mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                    Some(claimed) => {
                        claimed.requests.remove(&request);
                        claimed.requests.insert(request);
                        format!("room_name: {}, added new request {:?}", room_name, request_js)
                    }
                    _ => {
                        format!("room: {} is not claimed room", room_name)
                    }
                }
            }),
            Err(error) => {
                format!("incorrect room name: {}", error)
            }
        },
        Err(error) => {
            format!("incorrect request data: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn claim_room(room_name: String) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => {
            let room = game::rooms().get(room_name).expect("expect valid room");

            for structure in room.find(find::STRUCTURES, None) {
                let _ = match structure {
                    StructureObject::StructureTower(tower) if !tower.my() => tower.destroy(),
                    StructureObject::StructureSpawn(s) if !s.my() => s.destroy(),
                    StructureObject::StructureExtension(e) if !e.my() => e.destroy(),
                    StructureObject::StructureLink(link) if !link.my() => link.destroy(),
                    StructureObject::StructureLab(lab) if !lab.my() => lab.destroy(),
                    StructureObject::StructureObserver(o) if !o.my() => o.destroy(),
                    StructureObject::StructurePowerSpawn(ps) if !ps.my() => ps.destroy(),
                    StructureObject::StructureNuker(n) if !n.my() => n.destroy(),
                    _ => Ok(()),
                };
            }
            GLOBAL_MEMORY.with(|mem_refcell| {
                let _ = mem_refcell.borrow_mut().rooms.insert(room.name(), RoomState::default());
                format!("added room: {}", room.name())
            })
        }
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn requests(room_name: String) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => {
            GLOBAL_MEMORY.with(|mem_refcell| match mem_refcell.borrow().rooms.get(&room_name) {
                Some(claimed) => {
                    let mut result = format!("room: {} requests: \n", room_name);
                    for (i, request) in claimed.requests.iter().enumerate() {
                        result.extend_one(format!("{}: {} \n", i, request));
                    }
                    result
                }
                _ => {
                    format!("room: {} is not claimed room", room_name)
                }
            })
        }
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn resolve_request(room_name: String, request: JsValue) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => match serde_wasm_bindgen::from_value::<Request>(request.clone()) {
            Ok(request) => GLOBAL_MEMORY.with(|mem_refcell| {
                if let Some(memory) = mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                    if memory.requests.remove(&request) {
                        format!("resolved request(deleted): {:?}", request)
                    } else {
                        format!("can't found request: {:?}", request)
                    }
                } else {
                    format!("room: {} is not claimed room", room_name)
                }
            }),
            Err(err) => format!("incorrect request: {}", err),
        },
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn ccm(creep_name: String, creep_memory: JsValue) -> String {
    match serde_wasm_bindgen::from_value::<CreepMemory>(creep_memory.clone()) {
        Ok(new_memory) => GLOBAL_MEMORY.with(|shard_state| {
            match shard_state.borrow_mut().creeps.entry(creep_name.clone()) {
                std::collections::hash_map::Entry::Occupied(mut o) => {
                    o.insert(new_memory);
                    format!("changed creep {} memory to : {:?}", creep_name, creep_memory)
                }
                std::collections::hash_map::Entry::Vacant(_) => {
                    format!("incorrect creep name: {}", creep_name)
                }
            }
        }),
        Err(error) => {
            format!("incorrect creep memory: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn kill(name: String) -> bool {
    GLOBAL_MEMORY.with(|mem_refcell| {
        if let Some(creep_memory) = mem_refcell.borrow_mut().creeps.get_mut(&name) {
            creep_memory.respawned = true;

            if let Some(creep) = game::creeps().get(name) {
                let _ = creep.say("shmyak", false);
                return creep.suicide().ok().is_some();
            }
            return false;
        }
        false
    })
}

#[wasm_bindgen]
pub fn add_farm(room_name: String, remote_name: String) -> String {
    match RoomName::from_str(&room_name) {
        Ok(home_room) => match RoomName::from_str(&remote_name) {
            Ok(remote_room) => GLOBAL_MEMORY.with(|mem_refcell| {
                match mem_refcell.borrow_mut().rooms.get_mut(&home_room) {
                    Some(claimed) => {
                        if game::rooms().get(remote_room).is_some() {
                            let farm = FarmInfo::default();
                            claimed.farms.insert(remote_room, farm);
                            format!("room: {} added new remote: {}", room_name, remote_name)
                        } else {
                            format!("remote: {} is not available right now", remote_name)
                        }
                    }
                    _ => {
                        format!("room: {} is not claimed room", room_name)
                    }
                }
            }),
            Err(err) => format!("incorrect remote room name: {}", err),
        },
        Err(error) => format!("incorrect room name: {}", error),
    }
}

#[wasm_bindgen]
pub fn add_boost(room_name: String, boost: u8, timeout: u32) -> String {
    info!("room_name: {}, add boost: {}", room_name, boost);
    match RoomName::from_str(&room_name) {
        Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
            match mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                Some(claimed) => {
                    let boost_reason = match boost {
                        0 => BoostReason::Invasion,
                        1 => BoostReason::Upgrade,
                        2 => BoostReason::Repair,
                        3 => BoostReason::Dismantle,
                        4 => BoostReason::Caravan,
                        5 => BoostReason::Carry,
                        _ => BoostReason::Pvp,
                    };
                    claimed.boosts.insert(boost_reason, game::time() + timeout);
                    format!("added {} to room: {}", boost, room_name)
                }
                _ => {
                    format!("room: {} is not claimed room", room_name)
                }
            }
        }),
        Err(err) => format!("incorrect room name: {}", err),
    }
}

#[wasm_bindgen]
pub fn delete_boost(room_name: String, boost: u8) -> String {
    info!("room_name: {}, delete boost: {}", room_name, boost);
    match RoomName::from_str(&room_name) {
        Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
            match mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                Some(claimed) => {
                    let boost_reason = match boost {
                        0 => BoostReason::Invasion,
                        1 => BoostReason::Upgrade,
                        2 => BoostReason::Repair,
                        3 => BoostReason::Dismantle,
                        4 => BoostReason::Caravan,
                        5 => BoostReason::Carry,
                        _ => BoostReason::Pvp,
                    };
                    claimed.boosts.remove(&boost_reason);
                    format!("deleted {} from room: {}", boost, room_name)
                }
                _ => {
                    format!("room: {} is not claimed room", room_name)
                }
            }
        }),
        Err(err) => format!("incorrect room name: {}", err),
    }
}

#[wasm_bindgen]
pub fn trade(
    room_name: String,
    order_type: OrderType,
    resource: ResourceType,
    amount: u32,
    price: f64,
) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
            match mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                Some(claimed) => {
                    let trade = TradeData::with_price_and_amount(
                        order_type,
                        resource,
                        OrderedFloat(price),
                        amount,
                    );
                    claimed.trades.insert(trade);
                    format!("room: {} added trade: {:?}", room_name, trade)
                }
                _ => {
                    format!("room: {} is not claimed room", room_name)
                }
            }
        }),
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn clear_trades(room_name: String) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
            match mem_refcell.borrow_mut().rooms.get_mut(&room_name) {
                Some(claimed) => {
                    claimed.trades.retain(|_: &TradeData| false);
                    format!("clear all trades for: {}", room_name)
                }
                _ => {
                    format!("room: {} is not claimed room", room_name)
                }
            }
        }),
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn avoid_room(room_name: String) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
            mem_refcell.borrow_mut().avoid_rooms.insert(room_name, u32::MAX);
            format!("add room: {} to avoid set!", room_name)
        }),
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn get_plan_for(room_name: String, x: u8, y: u8) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => GLOBAL_MEMORY.with(|mem_refcell| {
            if let Some(memory) = mem_refcell.borrow_mut().rooms.get(&room_name) {
                if let Some(plan) = &memory.plan {
                    let xy = unsafe { RoomXY::unchecked_new(x, y) };
                    let result = plan.find_by_xy(xy).fold(
                        String::from_str("cells: ").expect("expect str"),
                        |acc, elem| {
                            format!(
                                "{}, [{:?}: {}, {}]",
                                acc,
                                elem.structure,
                                elem.xy.x.u8(),
                                elem.xy.y.u8()
                            )
                        },
                    );
                    return result;
                }
            }
            format!("memory: {} not found!", room_name)
        }),
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn add_plan(
    room_name: String,
    x: u8,
    y: u8,
    structure: JsValue,
    lvl: u8,
    r_lvl: Option<u8>,
) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => match serde_wasm_bindgen::from_value::<RoomStructure>(structure) {
            Ok(structure) => GLOBAL_MEMORY.with(|mem_refcell| {
                let xy = unsafe { RoomXY::unchecked_new(x, y) };

                let cell = PlannedCell::new(xy, structure, lvl, r_lvl);

                mem_refcell.borrow_mut().rooms.entry(room_name).and_modify(|memory| {
                    if let Some(mut plan) = memory.plan.take() {
                        plan.add_cell(cell);
                        memory.plan = Some(plan);
                    }
                });
                format!("added: {:?} at {} in {}", structure, xy, room_name)
            }),
            Err(err) => {
                format!("incorrect structure: {}", err)
            }
        },
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}

#[wasm_bindgen]
pub fn delete_plan_for(room_name: String, x: u8, y: u8, structure: JsValue) -> String {
    match RoomName::from_str(&room_name) {
        Ok(room_name) => match serde_wasm_bindgen::from_value::<RoomStructure>(structure) {
            Ok(structure) => GLOBAL_MEMORY.with(|mem_refcell| {
                let xy = unsafe { RoomXY::unchecked_new(x, y) };

                let cell = PlannedCell::new(xy, structure, 1, None);

                mem_refcell.borrow_mut().rooms.entry(room_name).and_modify(|memory| {
                    if let Some(mut plan) = memory.plan.take() {
                        plan.delete(cell);
                        memory.plan = Some(plan);
                    }
                });
                format!("deleted: {:?} at {} in {}", structure, xy, room_name)
            }),
            Err(err) => {
                format!("incorrect structure: {}", err)
            }
        },
        Err(error) => {
            format!("incorrect room name: {}", error)
        }
    }
}
