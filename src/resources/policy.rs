// use screeps::ResourceType;

// use crate::rooms::{RoomEvent, state::requests::{FactoryData, Request, RequestKind, assignment::Assignment}};

// #[derive(Clone, Copy)]
// pub enum Importance {
//     Critical,
//     Important,
//     NiceToHave,
// }

// #[derive(Clone, Copy)]
// pub struct ResourcePolicy {
//     pub low: Option<u32>,
//     pub high: Option<u32>,
//     pub importance: Importance,
// }

// impl ResourcePolicy {
//     pub fn is_low(&self, amount: u32) -> bool {
//         self.low.map_or(false, |low| amount < low)
//     }

//     pub fn is_high(&self, amount: u32) -> bool {
//         self.high.map_or(false, |high| amount > high)
//     }

//     pub fn decide_event(&self, res: ResourceType, amount: u32) -> Option<RoomEvent> {
//         //todo check battery
//         // if self.is_low(amount) {
//         //     // Example: low energy → produce energy
//         //     return match res {
//         //         ResourceType::Energy => Some(RoomEvent::Request(Request::new(
//         //                 RequestKind::Factory(FactoryData::new(ResourceType::Energy, 50000)),
//         //                 Assignment::None))),
//         //         // other low events for other resources
//         //         _ => None,
//         //     };
//         // }

//         //todo check ctrl lvl

//         // if self.is_high(amount) {
//         //     // Example: too much energy → produce battery
//         //     return match res {
//         //         ResourceType::Energy => Some(RoomEvent::Request(Request::new(
//         //                 RequestKind::Factory(FactoryData::new(ResourceType::Battery, 5000)),
//         //                 Assignment::None))),
//         //         // other high events
//         //         _ => None,
//         //     };
//         // }

//         None
//     }
// }

// pub fn resource_policy(res: ResourceType) -> Option<ResourcePolicy> {
//     use Importance::*;
//     Some(match res {
//         ResourceType::Energy => ResourcePolicy {
//             low: Some(10_000),
//             high: Some(200_000),
//             importance: Critical,
//         },
//         ResourceType::Battery => ResourcePolicy {
//             low: Some(2_000),
//             high: None,
//             importance: Important,
//         },
//         // … other resources you care about
//         _ => return None, // resources we don't manage
//     })
// }
