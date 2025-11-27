use js_sys::JsString;
use log::*;
use screeps::{game::{self, market::Order}, HasId, MarketResourceType, OrderType,
    ResourceType, RoomName, StructureTerminal};
use std::{cmp, iter::once, str::FromStr, collections::HashSet};
use crate::{
    rooms::{
        state::{
            RoomState, TradeData,
            requests::{Request, RequestKind, assignment::Assignment, meta::Status, CarryData}
        },
        RoomEvent, wrappers::claimed::Claimed
    }
};

impl Claimed {
    pub(crate) fn run_terminal(
        &self,
        requests: &HashSet<Request>,
        room_memory: &RoomState,
        orders: &[Order]) -> Option<RoomEvent>
    {
        let Some(terminal) = &self.terminal else {
            return None
        };

        let is_active_request = requests.iter()
            .any(|r| matches!(r.kind, RequestKind::Transfer(_)) &&
                matches!(r.status(), Status::InProgress | Status::OnHold));

        (terminal.cooldown() == 0)
            .then(|| self.try_trade(terminal, &room_memory.trades, orders))
            .flatten()
            .or_else(|| (!is_active_request)
                .then(|| {
                    get_request(requests)
                        .map(|mut request| {
                            request.join(None, None);
                            RoomEvent::ReplaceRequest(request)
                        })
                        .or_else(|| self.unload(
                            terminal,
                            //todo remove mineral when implemented sell minerals
                            &once(self.mineral.mineral_type())
                                .chain(trade_resources(&room_memory.trades))
                                .collect::<Vec<ResourceType>>()
                            ))
                        .or_else(|| {
                            let energy = terminal.store().get_used_capacity(Some(ResourceType::Energy));
                            (energy < 10000)
                                .then(|| self.supply_resources(
                                    terminal.raw_id(),
                                    ResourceType::Energy,
                                    10000 - energy))
                                .flatten()
                        })
                })
                .flatten()
            )
        
        // if terminal.cooldown() == 0 &&
        //     let Some(event) = self.try_trade(terminal, &room_memory.trades, orders)
        // {
        //     Some(event)
        // } else if !is_active_request {
        //     if let Some(mut request) = get_request(requests) && *request.status() == Status::Created {
        //         request.join(None, None);
        //         Some(RoomEvent::ReplaceRequest(request))
        //     } else if let Some(unload_event) =
        //         self.unload(
        //             terminal,
        //             //todo remove mineral when implemented sell minerals
        //             &once(self.mineral.mineral_type())
        //                 .chain(trade_resources(&room_memory.trades))
        //                 .collect::<Vec<ResourceType>>()
        //             )
        //     {
        //         Some(unload_event)
        //     } else {
        //         let energy = terminal.store().get_used_capacity(Some(ResourceType::Energy));
        //         if energy < 10000 && let Some(load_event) = self.supply_resources(
        //             terminal.raw_id(), ResourceType::Energy, 10000 - energy)
        //         {
        //             Some(load_event)
        //         } else {
        //             None
        //         }
        //     }
        // } else {
        //     None
        // }
    }

    fn try_trade(&self, terminal: &StructureTerminal, trades: &HashSet<TradeData>, orders: &[Order]) -> Option<RoomEvent> {
        trades.iter()
            .find_map(|trade_order| {
                let all = terminal.store().get_used_capacity(Some(trade_order.resource));
                if trade_order.amount > 0 {
                    if all >= trade_order.amount {
                        match trade_order.order_type {
                            OrderType::Buy if let Some(order) = find_appropriate_lowest_price_order(self.get_name(), orders, OrderType::Sell, trade_order.resource) => {
                                let amount = cmp::min(trade_order.amount, order.amount);
                                debug!("lowest order: {:?}, trade amount: {}", order, amount);
                                if order.summary <= *trade_order.price {
                                    Some(RoomEvent::Buy(order.id, trade_order.resource, amount))
                                } else {
                                    None
                                }
                            },
                            OrderType::Sell if let Some(order) = find_appropriate_highest_price_order(self.get_name(), orders, OrderType::Buy, trade_order.resource) => {
                                let amount = cmp::min(trade_order.amount, order.amount);
                                debug!("highest order: {:?}, trade amount: {}", order, amount);
                                if order.summary >= *trade_order.price {
                                    Some(RoomEvent::Sell(order.id, trade_order.resource, amount))
                                } else {
                                    None
                                }
                            },
                            _ => None
                        }
                    } else if let Some(storage) = self.storage() {
                        let amount = cmp::min(storage.store().get_used_capacity(Some(trade_order.resource)), trade_order.amount - all);
                        if amount > 0 {
                            Some(RoomEvent::Request(Request::new(
                                RequestKind::Carry(CarryData::new(
                                    storage.raw_id(),
                                    terminal.raw_id(),
                                    trade_order.resource,
                                    amount)),
                                Assignment::Single(None))))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    }
}

fn trade_resources(trades: &HashSet<TradeData>) -> impl Iterator<Item = ResourceType> + use<'_> {
    trades.iter()
        .filter_map(|trade_data| {
            if trade_data.amount > 0 {
                Some(trade_data.resource)
            } else {
                None
            }
        })
}

#[derive(Debug)]
struct OrderWithTransactionCost {
    // pub order: &'a Order,
    id: JsString,
    // resource: MarketResourceType,
    amount: u32,
    // price: f64,
    summary: f64
}

fn find_appropriate_highest_price_order(room_name: RoomName, orders: &[Order], order_type: OrderType, resource: ResourceType) -> Option<OrderWithTransactionCost> {
    orders.iter()
        .filter(|order| order.room_name().is_some() && order.order_type() == order_type &&
            order.resource_type() == MarketResourceType::Resource(resource))
        .map(|order| {
            let cost = game::market::calc_transaction_cost(
                order.amount(),
                &JsString::from_str(room_name.to_string().as_str()).expect("expect claimed room_name"),
                &order.room_name().expect("expect order room_name")
            );

            OrderWithTransactionCost {
                id: order.id(),
                amount: order.amount(),
                summary: (order.price() * order.amount() as f64 - cost as f64) / order.amount() as f64,
            }
        })
        .fold(None, |acc, item| {
            if let Some(acc) = acc {
                if acc.summary > item.summary {
                    Some(acc)
                } else {
                    Some(item)
                }
            } else {
                Some(item)
            }
        })
}

fn find_appropriate_lowest_price_order(room_name: RoomName, orders: &[Order], order_type: OrderType, resource: ResourceType) -> Option<OrderWithTransactionCost> {
    orders.iter()
        .filter(|order| order.room_name().is_some() && order.order_type() == order_type &&
            order.resource_type() == MarketResourceType::Resource(resource))
        .map(|order| {
            let cost = game::market::calc_transaction_cost(
                order.amount(),
                &JsString::from_str(room_name.to_string().as_str()).expect("expect claimed room_name"),
                &order.room_name().expect("expect order room_name")
            );

            OrderWithTransactionCost {
                id: order.id(),
                amount: order.amount(),
                summary: (order.price() * order.amount() as f64 + cost as f64) / order.amount() as f64,
            }
        })
        .fold(None, |acc, item| {
            if let Some(acc) = acc {
                if acc.summary < item.summary {
                    Some(acc)
                } else {
                    Some(item)
                }
            } else {
                Some(item)
            }
        })
}

fn get_request(requests: &HashSet<Request>) -> Option<Request> {
    requests.iter()
        .find(|r| matches!(r.kind, RequestKind::Transfer(_)) &&
            matches!(r.status(), Status::Created))
        .cloned()
}
