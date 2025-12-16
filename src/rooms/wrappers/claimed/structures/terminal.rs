use std::cmp;
use std::str::FromStr;

use js_sys::JsString;
use log::debug;
use screeps::game::{self, market::Order};
use screeps::{HasId, MarketResourceType, OrderType, ResourceType, RoomName, StructureTerminal};

use crate::rooms::{
    RoomEvent,
    shelter::Shelter,
    state::requests::{CarryData, Request, RequestKind, assignment::Assignment, meta::Status},
};

impl Shelter<'_> {
    pub(crate) fn run_terminal(&self, orders: &[Order]) -> Option<RoomEvent> {
        let terminal = self.base.terminal()?;

        (terminal.cooldown() == 0).then(|| self.try_trade(terminal, orders)).flatten().or_else(
            || {
                (!self.is_terminal_busy())
                    .then(|| {
                        self.get_terminal_request()
                            .map(|mut request| {
                                request.join(None, None);
                                RoomEvent::ReplaceRequest(request)
                            })
                            .or_else(|| {
                                self.unload(
                                    terminal,
                                    &self
                                        .get_trades()
                                        .map(|trade| trade.resource)
                                        .collect::<Vec<_>>(),
                                )
                            })
                            .or_else(|| {
                                let energy =
                                    terminal.store().get_used_capacity(Some(ResourceType::Energy));
                                (energy < 10000)
                                    .then(|| {
                                        self.supply_resources(
                                            terminal.raw_id(),
                                            ResourceType::Energy,
                                            10000 - energy,
                                        )
                                    })
                                    .flatten()
                            })
                    })
                    .flatten()
            },
        )
    }

    fn get_terminal_request(&self) -> Option<Request> {
        self.requests()
            .find(|r| {
                matches!(r.kind, RequestKind::Transfer(_)) && matches!(r.status(), Status::Created)
            })
            .cloned()
    }

    fn is_terminal_busy(&self) -> bool {
        self.state.requests.iter().any(|r| {
            matches!(r.kind, RequestKind::Transfer(_))
                && matches!(r.status(), Status::InProgress | Status::OnHold | Status::Finishing)
        })
    }

    fn try_trade(&self, terminal: &StructureTerminal, orders: &[Order]) -> Option<RoomEvent> {
        self.get_trades().find_map(|trade_order| {
            let all = terminal.store().get_used_capacity(Some(trade_order.resource));
            if trade_order.amount > 0 {
                if all >= trade_order.amount {
                    match trade_order.order_type {
                        OrderType::Buy
                            if let Some(order) = find_appropriate_lowest_price_order(
                                self.name(),
                                orders,
                                OrderType::Sell,
                                trade_order.resource,
                            ) =>
                        {
                            let amount = cmp::min(trade_order.amount, order.amount);
                            debug!("lowest order: {:?}, trade amount: {}", order, amount);
                            if order.summary <= *trade_order.price {
                                Some(RoomEvent::Buy(order.id, trade_order.resource, amount))
                            } else {
                                None
                            }
                        }
                        OrderType::Sell
                            if let Some(order) = find_appropriate_highest_price_order(
                                self.name(),
                                orders,
                                OrderType::Buy,
                                trade_order.resource,
                            ) =>
                        {
                            let amount = cmp::min(trade_order.amount, order.amount);
                            debug!("highest order: {:?}, trade amount: {}", order, amount);
                            if order.summary >= *trade_order.price {
                                Some(RoomEvent::Sell(order.id, trade_order.resource, amount))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                } else if let Some(storage) = self.storage() {
                    let amount = cmp::min(
                        storage.store().get_used_capacity(Some(trade_order.resource)),
                        trade_order.amount - all,
                    );
                    if amount > 0 {
                        Some(RoomEvent::Request(Request::new(
                            RequestKind::Carry(CarryData::new(
                                storage.raw_id(),
                                terminal.raw_id(),
                                trade_order.resource,
                                amount,
                            )),
                            Assignment::Single(None),
                        )))
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

#[derive(Debug)]
struct OrderWithTransactionCost {
    // pub order: &'a Order,
    id: JsString,
    // resource: MarketResourceType,
    amount: u32,
    // price: f64,
    summary: f64,
}

fn find_appropriate_highest_price_order(
    room_name: RoomName,
    orders: &[Order],
    order_type: OrderType,
    resource: ResourceType,
) -> Option<OrderWithTransactionCost> {
    orders
        .iter()
        .filter(|order| {
            order.room_name().is_some()
                && order.order_type() == order_type
                && order.resource_type() == MarketResourceType::Resource(resource)
        })
        .map(|order| {
            let cost = game::market::calc_transaction_cost(
                order.amount(),
                &JsString::from_str(room_name.to_string().as_str())
                    .expect("expect claimed room_name"),
                &order.room_name().expect("expect order room_name"),
            );

            OrderWithTransactionCost {
                id: order.id(),
                amount: order.amount(),
                summary: (order.price() * f64::from(order.amount()) - f64::from(cost))
                    / f64::from(order.amount()),
            }
        })
        .fold(None, |acc, item| {
            if let Some(acc) = acc {
                if acc.summary > item.summary { Some(acc) } else { Some(item) }
            } else {
                Some(item)
            }
        })
}

fn find_appropriate_lowest_price_order(
    room_name: RoomName,
    orders: &[Order],
    order_type: OrderType,
    resource: ResourceType,
) -> Option<OrderWithTransactionCost> {
    orders
        .iter()
        .filter(|order| {
            order.room_name().is_some()
                && order.order_type() == order_type
                && order.resource_type() == MarketResourceType::Resource(resource)
        })
        .map(|order| {
            let cost = game::market::calc_transaction_cost(
                order.amount(),
                &JsString::from_str(room_name.to_string().as_str())
                    .expect("expect claimed room_name"),
                &order.room_name().expect("expect order room_name"),
            );

            OrderWithTransactionCost {
                id: order.id(),
                amount: order.amount(),
                summary: (order.price() * f64::from(order.amount()) + f64::from(cost))
                    / f64::from(order.amount()),
            }
        })
        .fold(None, |acc, item| {
            if let Some(acc) = acc {
                if acc.summary < item.summary { Some(acc) } else { Some(item) }
            } else {
                Some(item)
            }
        })
}
