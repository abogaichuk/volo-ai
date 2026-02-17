use screeps::CREEP_RANGED_ACTION_RANGE;

pub const MIN_STORAGE_FREE_CAPACITY: i32 = 5_000;

pub const MIN_ENERGY_CAPACITY: u32 = 10_000;
pub const MAX_POWER_CAPACITY: u32 = 150_000;
// pub const CARRY_REQUEST_AMOUNT: u32 = 1000;

pub const MIN_CARRY_REQUEST_AMOUNT: u32 = 1_000;
pub const MAX_CARRY_REQUEST_AMOUNT: u32 = 5_000;
pub const MY_ROOMS_PICKUP_RESOURCE_THRESHOLD: u32 = 1000;
pub const FARM_ROOMS_PICKUP_RESOURCE_THRESHOLD: u32 = 1600;

pub const DEPOSIT_REQUEST_THRESHOLD: u32 = 35;

pub const MIN_PERIMETR_HITS: u32 = 100_000;
// pub const MIN_PERIMETR_HITS: u32 = 1000000;
pub const MAX_RAMPART_HITS: u32 = 299_000_000;
pub const MAX_WALL_HITS: u32 = 100_000;

pub const ROOM_NUMBER_RE: &str = r"^[WE]([0-9]+)[NS]([0-9]+)$";
/// Won't do pathing for moving creeps if current-tick CPU spend is above this
/// level when movement step is reached
pub const HIGH_CPU_THRESHOLD: f64 = 200.;
/// Won't do pathing for moving creeps if bucket is below this number
pub const LOW_BUCKET_THRESHOLD: i32 = 500;
/// Consider creeps to be stuck and get them a new path after this many ticks
pub const STUCK_REPATH_THRESHOLD: u8 = 3;
/// Limit for pathfinder ops
pub const MAX_OPS: u32 = 10_000;
/// Limit for pathfinder rooms
pub const MAX_ROOMS: u8 = 16;
/// A* heuristic weight - default is 1.2, but it risks non-optimal paths, so we
/// turn it down a bit
pub const HEURISTIC_WEIGHT: f64 = 1.0;
/// When task finding fails, idle this long
pub const NO_TASK_IDLE_TICKS: u32 = 1;
/// Creeps are just out of range of their ranged action at this range; at this
/// range they'll usually path avoiding creeps
pub const RANGED_OUT_OF_RANGE: u32 = (CREEP_RANGED_ACTION_RANGE + 1) as u32;
/// Creeps are just out of range of their melee action at this range; at this
/// range they'll usually path avoiding creeps
pub const MELEE_OUT_OF_RANGE: u32 = 2;
/// Creeps run away from edge range while escaping
pub const ESCAPE_FROM_EDGE_RANGE: u32 = 5;
/// When escaping from hostile, idle this long
pub const ESCAPE_IDLE_TICKS: u32 = 5;
/// When hiding in another room, wait this long
pub const HIDE_TIMEOUT: u32 = 10;

/// Handyman role considers energy on the ground for grabbing above this amount
pub const HANDYMAN_ENERGY_PICKUP_THRESHOLD: u32 = 600;
/// Handyman role considers energy in a structure for grabbing above this amount
pub const HANDYMAN_ENERGY_WITHDRAW_THRESHOLD: u32 = 500;

/// Hostile room avoidance timeout
pub const AVOID_HOSTILE_ROOM_TIMEOUT: u32 = 100_000;

pub const LONG_RANGE_ACTION: u32 = CREEP_RANGED_ACTION_RANGE as u32;
pub const CLOSE_RANGE_ACTION: u32 = 1;
