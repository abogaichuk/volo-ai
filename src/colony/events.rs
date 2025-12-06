use log::*;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::{BTreeMap, HashMap}};
use screeps::{Deposit, ObjectId, Position, RawObjectId, ResourceType, RoomName, StructurePowerBank, game};

use crate::{colony::orders::{CaravanOrder, DepositOrder, PowerbankOrder, ProtectOrder, ResourceOrder, WithdrawOrder}, resources::{chain_config::factory_chain_config, lack_handler_for}, statistics::RoomStats, utils::constants::AVOID_HOSTILE_ROOM_TIMEOUT};

use super::{
    less_cga, less_power, most_ctrl_lvl, most_money, prefered_room, Assignment, CaravanData, Claimed,
    ColonyOrder, DepositData, GlobalState, LRWData, Movement, PowerbankData, ProtectData, Request,
    RequestKind, TransferData,
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
    Stats(RoomName, RoomStats),
    BlackList(String)
}

pub(crate) struct ColonyContext<'a> {
    movement: Movement,
    bases: &'a HashMap<RoomName, Claimed>,
    warehouse: Option<(RoomName, u8)>,
}

impl <'a> ColonyContext<'a> {
    pub(super) fn new(movement: Movement, bases: &'a HashMap<RoomName, Claimed>) -> Self {
        let warehouse = bases
            .values()
            .map(|base| (base.get_name(), base.factory()
                .map(|f| f.level())
                .unwrap_or_default()))
            .min_by_key(|(_, lvl)| Reverse(*lvl));

        Self {
            movement,
            bases,
            warehouse,
        }
    }

    pub fn bases(&self) -> &HashMap<RoomName, Claimed> {
        self.bases
    }

    fn find_base_by_factory_level(&self, res: ResourceType) -> Option<RoomName> {
        factory_chain_config(res)
            .map(|config| config.random_chain().f_lvl)
            .and_then(|f_lvl| self.bases.iter()
                .find_map(|(room_name, base)| {
                    if base.factory().is_some_and(|f| f.level() == f_lvl) {
                        Some(*room_name)
                    } else {
                        None
                    }
                }))
    }
}

impl ColonyEvent {
    pub(super) fn assign(self, state: &mut GlobalState, context: &ColonyContext) {
        let movement = &context.movement;
        let bases = &context.bases;

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
                    if let Some(to_room) = context.find_base_by_factory_level(resource)
                        .or_else(|| context.warehouse.map(|w| w.0))
                        .filter(|rn| *rn != from)
                    {
                        excess_order.to = Some(to_room);
                        let transfer_request = Request::new(
                            RequestKind::Transfer(TransferData::new(
                                resource,
                                amount,
                                to_room,
                                Some(format!("colony assigned from: {}, to: {}", from, to_room)))),
                            Assignment::None);
                        info!("{} excess {}:{} transfer_to: {}", from, resource, amount, to_room);
                        state.add_request(from, transfer_request);
                    } else {
                        excess_order.to = Some(from);
                        info!("{} excess {}:{} alredy in warehouse!", from, resource, amount);
                    }
                    state.orders.insert(ColonyOrder::Excess(excess_order));
                }
            }
            ColonyEvent::Lack(from, resource, amount) => {
                let mut lack_order = ResourceOrder::new(from, resource, amount);
                if !state.orders.contains(&ColonyOrder::Lack(lack_order.clone())) {
                    if let Some(lack_result) =
                        lack_handler_for(resource)(resource, amount, context)
                    {
                        lack_order.to = Some(lack_result.room_name());
                        let transfer_request = Request::new(
                            RequestKind::Transfer(TransferData::new(
                                resource,
                                lack_result.amount(),
                                from,
                                Some(format!("colony assigned, sender: {} dest: {}", lack_result.room_name(), from)))),
                            Assignment::None);
                        info!("sender: {}, res {}:{} to: {}", lack_result.room_name(), resource, lack_result.amount(), from);
                        state.add_request(lack_result.room_name(), transfer_request);
                    } else {
                        info!("{} on low: {}:{}", from, resource, amount);
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
            ColonyEvent::Stats(name, stats) => {
                let _ = state.statistic.update(name, stats);
            }
        }
    }
}

fn notify_me(message: &str, interval: Option<u32>) {
    warn!("{}", message);
    game::notify(message, interval);
}