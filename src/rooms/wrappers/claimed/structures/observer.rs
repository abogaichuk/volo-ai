use log::*;
use rand::Rng;
use crate::rooms::wrappers::claimed::Claimed;

//e6s20 - e24s20
//add_to_observe('E6S20')
//remove_from_observe('E6S20')
//Game.getObjectById('675740745ef8fa1b1eae907a').observeRoom('W1S40')
impl Claimed {
    pub(crate) fn run_observer(&self) {
        if let Some(observer) = &self.observer {
            let x = get_random(-5, 5);
            let y = get_random(-5, 5);
    
            if let Some(target) = self.get_name().checked_add((x, y)) {
                let res = observer.observe_room(target);
                    match res {
                        Ok(_) => {},
                        Err(err) => error!("room: {} observation {} error: {:?}", self.get_name(), target, err)
                    }
            } else {
                error!("{} invalid observation coords: x:{}, y: {}", self.get_name(), x, y);
            }
        }
    }
}

fn get_random(from: i32, to: i32) -> i32 {
    rand::thread_rng().gen_range(from..=to)
}
