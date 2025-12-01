#![feature(extend_one, let_chains, if_let_guard, iter_next_chunk, int_roundings)]
use log::*;
use std::cell::RefCell;

use utils::commons;
use wasm_bindgen::prelude::*;
use rand::{SeedableRng, RngCore, rngs::StdRng};
use getrandom::register_custom_getrandom;
use screeps::game;
use crate::colony::GlobalState;

mod logging;
mod colony;
mod utils;
mod units;
mod rooms;
mod globals;
mod statistics;
mod movement;
mod resources;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

thread_local! {
    pub static GLOBAL_MEMORY: RefCell<GlobalState> = RefCell::new(GlobalState::load_or_default());
}

#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    let cpu_start = game::cpu::get_used();
    if cfg!(feature = "mmo") && game::cpu::bucket() == 10000 {
        let _ = game::cpu::generate_pixel();
    }

    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Info);
    });

    GLOBAL_MEMORY.with(|mem_refcell| {
        let mut colony = mem_refcell.borrow_mut();

        if colony.rooms.is_empty() {
            panic!("parsing error occured..");
        }

        colony.run_tick();
        colony.write()
    });
    debug!("loop done! cpu: {}", game::cpu::get_used() - cpu_start);
}

// implement a custom randomness generator for the getrandom crate,
// because the `js` feature expects the Node.js WebCrypto API to be available
// (it's not available in the Screeps Node.js environment)
fn custom_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    let mut rng = StdRng::seed_from_u64(js_sys::Math::random().to_bits());
    rng.fill_bytes(buf);
    Ok(())
}
register_custom_getrandom!(custom_getrandom);
