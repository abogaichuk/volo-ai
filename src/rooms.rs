use std::collections::{HashMap, HashSet};
use std::iter::Iterator;

use itertools::Itertools;
use js_sys::JsString;
use log::debug;
use screeps::{
    HasPosition, Mineral, PowerType, ResourceType, Room, RoomName, RoomPosition, RoomXY,
    StructureType, game,
};

use crate::commons::look_for;
use crate::rooms::shelter::Shelter;
use crate::rooms::state::constructions::{PlannedCell, RoomPlan, RoomStructure};
use crate::rooms::state::requests::Request;
use crate::rooms::state::{BoostReason, RoomState};
use crate::rooms::wrappers::farm::Farm;
use crate::rooms::wrappers::neutral::Neutral;
use crate::units::roles::Role;

pub mod shelter;
pub mod state;
pub mod wrappers;

#[derive(Debug)]
pub enum RoomEvent {
    Spawned(String, Role, usize),
    Spawn(Role, usize),
    MayBeSpawn(Role),
    CancelRespawn(Role),
    AddPower(PowerType),
    DeletePower(PowerType),
    AddBoost(BoostReason, u32),
    RetainBoosts,
    StopFarm(RoomName, Option<RoomName>),
    StartFarm(RoomName, Option<RoomName>),
    EditPlans(HashMap<RoomName, RoomPlan>),
    Plan(RoomPlan),
    ReplaceCell(PlannedCell),
    Construct(HashMap<RoomXY, StructureType>),
    IncrementPlanLvl,
    Lack(ResourceType, u32),
    Excess(ResourceType, u32),
    Avoid(RoomName, u32),
    // Sos, //if claimed room is attacked and has no power to defends by itself
    Request(Request),
    ReplaceRequest(Request),
    Sell(JsString, ResourceType, u32),
    Buy(JsString, ResourceType, u32),
    Intrusion(Option<String>),
    NukeFalling,
    Defend(RoomName),
    ActivateSafeMode(String),
    BlackList(String),
    UpdateStatistic,
}

pub fn register_rooms<'a>(
    states: &'a mut HashMap<RoomName, RoomState>,
    white_list: &'a HashSet<String>,
) -> (HashMap<RoomName, Shelter<'a>>, Vec<Neutral>) {
    let mut rooms: HashMap<RoomName, Room> = game::rooms().entries().collect();

    let mut homes = HashMap::new();
    for (room_name, state) in states.iter_mut() {
        if let Some(base_room) = rooms.remove(room_name) {
            let mut farms = Vec::new();

            for (farm_name, farm_info) in state.farms.iter() {
                if let Some(farm_room) = rooms.remove(farm_name) {
                    farms.push(Farm::new(farm_room, farm_info.clone()));
                }
            }

            homes.insert(*room_name, Shelter::new(base_room, farms, state, white_list));
        }
    }
    // for (room_name, state) in states.iter_mut() {
    //     if let Some(base_room) = rooms.remove(room_name) {
    //         let mut farms = Vec::new();

    //         for farm_name in state.farms.keys() {
    //             if let Some(farm_room) = rooms.remove(farm_name) {
    //                 farms.push(Farm::new(farm_room));
    //             }
    //         }

    //         homes.insert(*room_name, Shelter::new(base_room, farms, state, white_list));
    //     }
    // }
    debug!("registered shelters: {}", homes.len());
    (homes, rooms.into_values().map(Neutral::new).collect())
}

fn missed_buildings(
    room_name: RoomName,
    plan: &RoomPlan,
) -> impl Iterator<Item = (RoomXY, StructureType)> + use<'_> {
    plan.current_lvl_buildings()
        .sorted_by_key(|cell| cell.structure)
        .filter_map(move |cell| {
            if let Ok(str_type) = StructureType::try_from(cell.structure) {
                let room_position = RoomPosition::new(cell.xy.x.u8(), cell.xy.y.u8(), room_name);

                if look_for(&room_position, str_type) { None } else { Some((cell.xy, str_type)) }
            } else {
                None
            }
        })
        .sorted_by_key(|(xy, _)| *xy)
        .take(5)
}

fn is_extractor(mineral: &Mineral) -> bool {
    look_for(&mineral.pos().into(), screeps::StructureType::Extractor)
}
