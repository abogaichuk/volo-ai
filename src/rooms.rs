use itertools::Itertools;
use log::*;
use screeps::{
    game, HasPosition, Mineral, ObjectId, RoomPosition, PowerType,
    ResourceType, Room, RoomName, RoomXY, StructureLab, StructureType
};
use std::{collections::{HashMap, HashSet}, iter::Iterator};
use js_sys::JsString;
use crate::{
    commons::look_for,
    rooms::{
        shelter::Shelter,
        state::{BoostReason, RoomState, constructions::{LabStatus, PlannedCell, RoomPlan}, requests::{CreepHostile, Request}},
        wrappers::{claimed::Claimed, farm::Farm, neutral::Neutral}
    },
    units::roles::Role
};

pub mod wrappers;
pub mod shelter;
pub mod state;

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
    // UpdateLab(RoomXY, LabStatus),
    // UpdateLab(ObjectId<StructureLab>, LabStatus),
    StopFarm(RoomName, Option<RoomName>),
    StartFarm(RoomName, Option<RoomName>),
    AddPlans(HashMap<RoomName, RoomPlan>),
    Plan(RoomPlan),
    ReplaceCell(PlannedCell),
    // Build(RoomName, HashMap<RoomXY, StructureType>),
    BuiltAll,
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
    Defend(RoomName, Vec<CreepHostile>),
    ActivateSafeMode(String),
    BlackList(String),
    // #[default]
    // Nothing
}

pub fn register_rooms<'a>(
    states: &'a mut HashMap<RoomName, RoomState>,
    white_list: &'a HashSet<String>
) -> (HashMap<RoomName, Shelter<'a>>, Vec<Neutral>) {
    let mut rooms: HashMap<RoomName, Room> = game::rooms().entries().collect();

    let mut homes = HashMap::new();
    for (room_name, state) in states.iter_mut() {
        if let Some(base_room) = rooms.remove(room_name) {
            let mut farms = Vec::new();

            for farm_name in state.farms.keys() {
                if let Some(farm_room) = rooms.remove(farm_name) {
                    farms.push(Farm::new(farm_room));
                }
            }

            homes.insert(*room_name, Shelter::new(state, Claimed::new(base_room, farms), white_list));
        }
    }
    debug!("registered shelters: {}", homes.len());
    (homes, rooms.into_values().map(Neutral::new).collect())
}

fn missed_buildings(room_name: RoomName, plan: &RoomPlan) -> impl Iterator<Item = (RoomXY, StructureType)> + use<'_> {
    plan.current_lvl_buildings()
        .sorted_by_key(|cell| cell.structure)
        .filter_map(move |cell| {
            if let Ok(str_type) = StructureType::try_from(cell.structure) {
                let room_position = RoomPosition::new(
                    cell.xy.x.u8(),
                    cell.xy.y.u8(),
                    room_name);

                if !look_for(&room_position, str_type) {
                    Some((cell.xy, str_type))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .take(5)
        // .collect()
}

fn is_extractor(mineral: &Mineral) -> bool {
    look_for(&mineral.pos().into(), screeps::StructureType::Extractor)
}