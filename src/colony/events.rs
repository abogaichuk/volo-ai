use log::*;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::{BTreeMap, HashMap}};
use screeps::{Deposit, ObjectId, OrderType, Position, RawObjectId, ResourceType, RoomName, StructurePowerBank, game};

use crate::{colony::orders::{CaravanOrder, DepositOrder, PowerbankOrder, ProtectOrder, ResourceOrder, WithdrawOrder}, utils::constants::AVOID_HOSTILE_ROOM_TIMEOUT};

use super::{
    kinds, less_cga, less_power, most_ctrl_lvl, most_money, prefered_room, Assignment, CaravanData, Claimed,
    ColonyOrder, DepositData, GlobalState, Kinds, LRWData, Movement, PowerbankData, ProtectData, Request,
    RequestKind, TransferData, MAX_FACTORY_LEVEL,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum ColonyEvent {
    Excess(RoomName, ResourceType, u32),
    Lack(RoomName, ResourceType, u32),
    AvoidRoom(RoomName, u32),
    DeclareNew(RoomName),
    Caravan(BTreeMap<String, u32>, RoomName),
    Expansion(RoomName, u8, Option<String>, bool),
    Powerbank(ObjectId<StructurePowerBank>, Position, u32),
    Deposit(ObjectId<Deposit>, Position, usize),
    Withdraw(RawObjectId, Position, ResourceType, u32),
    Notify(String, Option<u32>),
    BlackList(String)
}

pub struct EventContext {
    movement: Movement,
    bases: HashMap<RoomName, Claimed>,
    warehouse: Option<RoomName>,
}

impl EventContext {
    pub(super) fn new(movement: Movement, bases: HashMap<RoomName, Claimed>) -> Self {
        let warehouse = bases
            .values()
            .filter(|base| base.factory().is_some_and(|f| f.level() > 0))
            .min_by_key(|base| Reverse(base.factory().map(|f| f.level()).unwrap_or_default()))
            .map(Claimed::get_name);

        Self {
            movement,
            bases,
            warehouse,
        }
    }
}

impl ColonyEvent {
    pub fn handle(self, state: &mut GlobalState, context: &EventContext) {
        let movement = &context.movement;
        let bases = &context.bases;
        let warehouse = context.warehouse;

        match self {
            ColonyEvent::AvoidRoom(room_name, timeout) => {
                state.avoid_rooms.insert(room_name, timeout);
            }
            ColonyEvent::DeclareNew(room_name) => {
                let _ = state.rooms.entry(room_name).or_default();
            }
            ColonyEvent::Caravan(creeps, from) => {
                let order = ColonyOrder::Caravan(CaravanOrder::new(creeps, from));
                if let Some(existed) = state.orders.take(&order) &&
                    let ColonyOrder::Caravan(mut caravan_order) = existed
                {
                    if let Some((base_name, ambush)) =
                        caravan_order.catch_caravan(from, bases, movement)
                    {
                        info!("caravan will be catched in: {}, by: {}", ambush, base_name);
                        caravan_order.room = Some(base_name);
                        state.add_request(base_name, Request::new(
                            RequestKind::Caravan(CaravanData::new(ambush)),
                            Assignment::Single(None)));
                    }
                    state.orders.insert(ColonyOrder::Caravan(caravan_order));
                } else {
                    state.orders.insert(order);
                }
            }
            ColonyEvent::Expansion(room_name, ctrl_lvl, username, safe_mode) => {
                if ctrl_lvl < 7 && !safe_mode && username
                    .is_some_and(|u| state.black_list.contains(&u))
                {
                    let mut order = ColonyOrder::Protect(ProtectOrder::new(room_name, ctrl_lvl));
                    if !state.orders.contains(&order) &&
                        let Some((base_name, _)) =
                            prefered_room(room_name, movement, bases.values(), most_ctrl_lvl)
                    {
                        if let ColonyOrder::Protect(protect_order) = &mut order {
                            protect_order.room = Some(base_name);
                        }
                        state.orders.insert(order);
                        state.add_request(
                            base_name,
                            Request::new(
                                RequestKind::Protect(ProtectData::new(room_name, ctrl_lvl)),
                                Assignment::Single(None)));
                    }
                } else {
                    state.avoid_rooms.insert(room_name, game::time() + AVOID_HOSTILE_ROOM_TIMEOUT);
                }
            }
            ColonyEvent::Deposit(id, pos, empty_cells) => {
                let mut order = ColonyOrder::Deposit(DepositOrder::new(id, pos, empty_cells));
                if !state.orders.contains(&order) &&
                    let Some((base_name, _)) =
                        prefered_room(pos.room_name(), movement, bases.values(), most_money)
                {
                    if let ColonyOrder::Deposit(deposit_order) = &mut order {
                        deposit_order.room = Some(base_name);
                    }
                    state.orders.insert(order);
                    state.add_request(
                        base_name,
                        Request::new(
                            RequestKind::Deposit(DepositData::new(id, pos, empty_cells)),
                            Assignment::Squads(Vec::new())));
                }
            }
            ColonyEvent::Powerbank(id, pos, amount) => {
                let mut order = ColonyOrder::Powerbank(PowerbankOrder::new(id, pos, amount));
                if !state.orders.contains(&order) &&
                    let Some((base_name, _)) =
                        prefered_room(pos.room_name(), movement, bases.values(), less_power)
                {
                    if let ColonyOrder::Powerbank(powerbank_order) = &mut order {
                        powerbank_order.room = Some(base_name);
                    }
                    state.orders.insert(order);
                    
                    let pb_data = if state.postponed_farms.contains(&pos.room_name()) {
                        PowerbankData::postponed(id, pos, amount)
                    } else {
                        PowerbankData::new(id, pos, amount)
                    };

                    state.add_request(
                        base_name,
                        Request::new(
                            RequestKind::Powerbank(pb_data),
                            Assignment::Squads(Vec::new())));
                }
            }
            ColonyEvent::Withdraw(id, pos, resource, amount) => {
                let mut order = ColonyOrder::Withdraw(WithdrawOrder::new(id, pos, resource, amount));
                if !state.orders.contains(&order) &&
                    let Some((base_name, _)) =
                        prefered_room(pos.room_name(), movement, bases.values(), less_cga)
                {
                    if let ColonyOrder::Withdraw(withdraw_order) = &mut order {
                        withdraw_order.room = Some(base_name);
                    }
                    state.orders.insert(order);
                    state.add_request(
                        base_name,
                        Request::new(
                            RequestKind::LongRangeWithdraw(LRWData::new(id, pos, resource, amount)),
                            Assignment::Single(None)));
                }
            }
            ColonyEvent::Excess(from, resource, amount) => {
                let mut excess_order = ResourceOrder::new(from, resource, amount);
                if !state.orders.contains(&ColonyOrder::Excess(excess_order.clone())) {
                    let f_lvl = factory_level_for(resource);
                    let kinds = kinds(resource);

                    if kinds.intersects(Kinds::TRADEABLE) && let Some(warehouse_room) = warehouse {
                        if warehouse_room != from {
                            excess_order.to = Some(warehouse_room);
                            let transfer_request = Request::new(
                                RequestKind::Transfer(TransferData::new(
                                    resource,
                                    amount,
                                    warehouse_room,
                                    Some(format!("colony assigned from: {}", from)))),
                                Assignment::None);
                            info!("{} added excess TRADEABLE: {:?} by colony", from, transfer_request);
                            state.add_request(from, transfer_request);   
                        } else {
                            warn!("{} try selling {}:{}", from, resource, amount);
                            state.trade(from, OrderType::Sell, resource, amount);
                        }
                    } else if kinds.intersects(Kinds::PRODUCEABLE) &&
                        let Some(to) = find_base_by_factory_lvl(bases, f_lvl) &&
                        to.get_name() != from
                    {
                        excess_order.to = Some(to.get_name());

                        let transfer_request = Request::new(
                            RequestKind::Transfer(TransferData::new(
                                resource,
                                amount,
                                to.get_name(),
                                Some(format!("colony assigned from: {}", from)))),
                            Assignment::None);
                        info!("{} added excess PRODUCEABLE: {:?} by colony", from, transfer_request);
                        state.add_request(from, transfer_request);
                    } else {
                        excess_order.to = Some(from);
                        info!("resource: {}:{} warehouse not found", resource, amount);
                    }
                    state.orders.insert(ColonyOrder::Excess(excess_order));
                }
            }
            ColonyEvent::Lack(from, resource, amount) => {
                let mut lack_order = ResourceOrder::new(from, resource, amount);
                if !state.orders.contains(&ColonyOrder::Lack(lack_order.clone())) {
                    let kinds = kinds(resource);
                    if kinds.intersects(Kinds::PRODUCEABLE) &&
                        let Some((room_name, found)) = find_resource(bases, from, resource)
                    {
                        lack_order.to = Some(room_name);
                        let transfer_request = Request::new(
                            RequestKind::Transfer(TransferData::new(
                                resource,
                                std::cmp::min(amount, found),
                                from,
                                Some(format!("colony assigned from: {}", from)))),
                            Assignment::None);
                        //todo bug here
                        //added lack PRODUCEABLE: Transfer(TransferRequest with 0 resource
                        info!("{} added lack PRODUCEABLE: {:?} by colony", room_name, transfer_request);
                        state.add_request(room_name, transfer_request);
                    } else if kinds.intersects(Kinds::STOREABLE) &&
                        let Some((room_name, found)) = find_resource(bases, from, resource) &&
                        found > amount && found > 6000
                    {
                        lack_order.to = Some(room_name);
                        let transfer_request = Request::new(
                            RequestKind::Transfer(TransferData::new(
                                resource,
                                amount,
                                from,
                                Some(format!("colony assigned from: {}", room_name)))),
                            Assignment::None);
                        info!("{} added lack STOREABLE: {:?} by colony", room_name, transfer_request);
                        state.add_request(room_name, transfer_request);
                    }
                    state.orders.insert(ColonyOrder::Lack(lack_order));
                }
            }
            ColonyEvent::Notify(message, interval) => {
                notify_me(&message, interval);
            }
            ColonyEvent::BlackList(username) => {
                state.black_list.insert(username);
            }
        }
    }
}

fn notify_me(message: &str, interval: Option<u32>) {
    warn!("{}", message);
    game::notify(message, interval);
}

fn factory_level_for(resource: ResourceType) -> u8 {
    match resource {
        ResourceType::Cell | ResourceType::Wire | ResourceType::Alloy | ResourceType::Condensate => 1,
        ResourceType::Phlegm | ResourceType::Switch | ResourceType::Tube | ResourceType::Concentrate => 2,
        ResourceType::Tissue | ResourceType::Transistor | ResourceType::Fixtures | ResourceType::Extract => 3,
        ResourceType::Muscle | ResourceType::Microchip | ResourceType::Frame | ResourceType::Spirit => 4,
        _ => MAX_FACTORY_LEVEL,
    }
}

fn find_resource(homes: &HashMap<RoomName, Claimed>, ignore: RoomName, resource: ResourceType) -> Option<(RoomName, u32)> {
    homes
        .values()
        .filter(|base| base.get_name() != ignore)
        .map(|base| (base.get_name(), base.resource_amount(resource)))
        .max_by_key(|(_, amount)| *amount)
}

fn find_base_by_factory_lvl(bases: &HashMap<RoomName, Claimed>, f_lvl: u8) -> Option<&Claimed> {
    bases
        .values()
        .find(|base| base.factory().is_some_and(|f| f.level() == f_lvl))
}