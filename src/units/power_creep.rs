use std::collections::HashMap;

use log::{error, warn, debug};
use screeps::{
    Effect, EffectType, HasPosition, Position, PowerCreep, PowerInfo, PowerType, ResourceType,
    RoomName, RoomObjectProperties, RoomXY, SharedCreepProperties, StructureController, game,
};
use serde::{Deserialize, Serialize};

use crate::movement::{Movement, MovementGoal, MovementGoalBuilder, MovementProfile, PathState};
use crate::rooms::shelter::Shelter;
use crate::utils::constants::{CLOSE_RANGE_ACTION, LONG_RANGE_ACTION};

pub fn run_power_creeps(
    states: &mut HashMap<String, PowerCreepMemory>,
    homes: &mut HashMap<RoomName, Shelter<'_>>,
    movement: &mut Movement,
) {
    let mut p_creeps: HashMap<String, PowerCreep> = game::power_creeps()
        .entries()
        .filter_map(|(name, apc)| PowerCreep::try_from(apc).ok().map(|pc| (name, pc)))
        .collect();

    for (name, memory) in states.iter_mut() {
        let pc = match p_creeps.remove(name) {
            Some(pc) => pc,
            _ => continue, //gc will clear them
        };

        if let Some(room_name) = memory.get_home().as_ref() {
            if let Some(mut unit) =
                homes.get_mut(room_name).map(|home| PcUnit { creep: pc, memory, home })
            {
                let goal = unit.run_unit();
                unit.move_to_goal(goal, movement);
            } else {
                error!("{} error creation pcunit!", name);
            }
        } else {
            memory.home = get_home(&pc, homes).map(|s| s.name());
        }
    }

    for (name, _) in p_creeps {
        states.insert(name, PowerCreepMemory::default());
    }
}

fn get_home<'a>(
    pc: &'a PowerCreep,
    homes: &'a HashMap<RoomName, Shelter>,
) -> Option<&'a Shelter<'a>> {
    pc.room().and_then(|room| {
        homes
            .values()
            .find(|base| base.name() == room.name())
            .or_else(|| find_closest_home(room.name(), homes))
    })
}

fn find_closest_home<'a>(
    target: RoomName,
    homes: &'a HashMap<RoomName, Shelter>,
) -> Option<&'a Shelter<'a>> {
    homes
        .values()
        .filter(|home| home.controller().level() > 1)
        .min_by_key(|home| game::map::get_room_linear_distance(home.name(), target, false))
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PowerCreepMemory {
    #[serde(skip)]
    pub home: Option<RoomName>,
    #[serde(skip)]
    pub path_state: Option<PathState>,
}

impl PowerCreepMemory {
    pub const fn get_home(&self) -> Option<&RoomName> {
        self.home.as_ref()
    }
}

pub struct PcUnit<'m, 'h, 's> {
    creep: PowerCreep,
    memory: &'m mut PowerCreepMemory,
    home: &'h mut Shelter<'s>,
}

impl PcUnit<'_, '_, '_> {
    fn name(&self) -> String {
        self.creep.name()
    }

    fn pos(&self) -> Position {
        self.creep.pos()
    }

    pub fn run_unit(&mut self) -> Option<MovementGoal> {
        if !self.home.controller().is_power_enabled() {
            return enable_controller(&self.creep, self.home.controller());
        }

        if self.creep.ticks_to_live().is_some_and(|ticks| ticks < 100) {
            return self.renew();
        }

        if self.is_power_available(PowerType::GenerateOps) {
            let _ = self.creep.use_power(PowerType::GenerateOps, None);
            return None;
        }

        if self.home.invasion() {
            if self.creep.store().get_used_capacity(Some(ResourceType::Ops)) < 10 {
                if let Some(storage) = self.home.storage()
                    && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 10
                {
                    if self.creep.pos().is_near_to(storage.pos()) {
                        let _ = self.creep.withdraw(storage, ResourceType::Ops, None);
                        None
                    } else {
                        build_goal(storage.pos(), CLOSE_RANGE_ACTION, None)
                    }
                } else {
                    warn!("room: {} resource ops not enough!!", self.home.name());
                    None
                }
            } else if self.is_power_available(PowerType::OperateTower) {
                if let Some(tower) = self.home.tower_without_effect() {
                    if self.creep.pos().in_range_to(tower.pos(), LONG_RANGE_ACTION) {
                        let _ = self.creep.use_power(PowerType::OperateTower, Some(tower));
                        None
                    } else {
                        build_goal(tower.pos(), LONG_RANGE_ACTION, None)
                    }
                } else {
                    None
                }
                //todo deside use fortify or operate towers firstly?
            } else if self.is_power_available(PowerType::Fortify) {
                //todo take from room history
                if let Some(rampart) = self.home.lowest_perimetr_hits() {
                    if self.creep.pos().in_range_to(rampart.pos(), LONG_RANGE_ACTION) {
                        //todo moving safe for powercreep
                        let _ = self.creep.use_power(PowerType::Fortify, Some(rampart));
                        None
                    } else {
                        build_goal(rampart.pos(), LONG_RANGE_ACTION, None)
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else if let (Some(source), Some(_)) =
            (self.home.source_without_effect(), self.get_power(PowerType::RegenSource))
        {
            if self.creep.pos().in_range_to(source.pos(), LONG_RANGE_ACTION) {
                let res = self.creep.use_power(PowerType::RegenSource, Some(source));
                match res {
                    Ok(()) => {}
                    Err(err) => {
                        error!("use power error: {:?}", err);
                    }
                }
                None
            } else {
                build_goal(source.pos(), LONG_RANGE_ACTION, None)
            }
        } else if let (Some(storage), Some(_)) =
            (self.home.full_storage_without_effect(), self.get_power(PowerType::OperateStorage))
        {
            if self.creep.store().get_used_capacity(Some(ResourceType::Ops)) < 100 {
                if let Some(storage) = self.home.storage()
                    && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 100
                {
                    if self.creep.pos().is_near_to(storage.pos()) {
                        let _ = self.creep.withdraw(storage, ResourceType::Ops, None);
                        None
                    } else {
                        build_goal(storage.pos(), CLOSE_RANGE_ACTION, None)
                    }
                } else {
                    warn!("room: {} resource ops not enough!!", self.home.name());
                    None
                }
            } else {
                debug!("creep full: {}", self.creep.name());
                if self.creep.pos().get_range_to(storage.pos()) <= LONG_RANGE_ACTION {
                    let res = self.creep.use_power(PowerType::OperateStorage, Some(storage));
                    debug!("creep {} operate storage res: {:?}", self.name(), res);
                    match res {
                        Ok(()) => {}
                        Err(err) => {
                            error!("use power error: {:?}", err);
                        }
                    }
                    None
                } else {
                    build_goal(storage.pos(), LONG_RANGE_ACTION, None)
                }
            }
        } else if self.home.mineral_without_effect()
            && self.get_power(PowerType::RegenMineral).is_some()
        {
            if self.creep.pos().in_range_to(self.home.mineral().pos(), 3) {
                let res = self.creep.use_power(PowerType::RegenMineral, Some(self.home.mineral()));
                match res {
                    Ok(()) => {}
                    Err(err) => {
                        error!("use power error: {:?}", err);
                    }
                }
                None
            } else {
                build_goal(self.home.mineral().pos(), LONG_RANGE_ACTION, None)
            }
        } else if let (Some(spawn), Some(_)) =
            (self.home.spawn_without_effect(), self.get_power(PowerType::OperateSpawn))
            && self.home.is_power_enabled(&PowerType::OperateSpawn)
        {
            if self.creep.store().get_used_capacity(Some(ResourceType::Ops)) < 100 {
                if let Some(storage) = self.home.storage()
                    && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 100
                {
                    if self.creep.pos().is_near_to(storage.pos()) {
                        let _ = self.creep.withdraw(storage, ResourceType::Ops, None);
                        None
                    } else {
                        build_goal(storage.pos(), CLOSE_RANGE_ACTION, None)
                    }
                } else {
                    warn!("room: {} resource ops not enough!!", self.home.name());
                    None
                }
            } else {
                debug!("creep full: {}", self.creep.name());
                if self.creep.pos().get_range_to(spawn.pos()) <= LONG_RANGE_ACTION {
                    let res = self.creep.use_power(PowerType::OperateSpawn, Some(spawn));
                    debug!("creep {} operate spawn res: {:?}", self.creep.name(), res);
                    match res {
                        Ok(()) => {}
                        Err(err) => {
                            error!("use power error: {:?}", err);
                        }
                    }
                    None
                } else {
                    build_goal(spawn.pos(), LONG_RANGE_ACTION, None)
                }
            }
        } else if let (Some(factory), Some(_), true) = (
            self.home.factory_without_effect(),
            self.get_power(PowerType::OperateFactory),
            self.home.is_power_enabled(&PowerType::OperateFactory),
        ) {
            if self.creep.store().get_used_capacity(Some(ResourceType::Ops)) < 100 {
                if let Some(storage) = self.home.storage()
                    && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 100
                {
                    if self.creep.pos().is_near_to(storage.pos()) {
                        let _ = self.creep.withdraw(storage, ResourceType::Ops, None);
                        None
                    } else {
                        build_goal(storage.pos(), CLOSE_RANGE_ACTION, None)
                    }
                } else {
                    warn!("room: {} resource ops not enough!!", self.home.name());
                    None
                }
            } else {
                debug!("creep full: {}", self.creep.name());
                if self.creep.pos().get_range_to(factory.pos()) <= LONG_RANGE_ACTION {
                    let res = self.creep.use_power(PowerType::OperateFactory, Some(factory));
                    debug!("creep {} operate storage res: {:?}", self.creep.name(), res);
                    match res {
                        Ok(()) => {}
                        Err(err) => {
                            error!("use power error: {:?}", err);
                        }
                    }
                    None
                } else {
                    build_goal(factory.pos(), LONG_RANGE_ACTION, None)
                }
            }
        } else if self.creep.store().get_used_capacity(Some(ResourceType::Ops))
            > self.creep.store().get_capacity(Some(ResourceType::Ops)) / 2
        {
            if let Some(storage) = self.home.storage()
                && storage.store().get_free_capacity(None) > 5000
            {
                if self.creep.pos().is_near_to(storage.pos()) {
                    let _ = self.creep.transfer(storage, ResourceType::Ops, None);
                    None
                } else {
                    build_goal(storage.pos(), CLOSE_RANGE_ACTION, None)
                }
            } else {
                warn!("room: {}, storage is full!!", self.home.name());
                None
            }
        } else if controller_without_effect(self.home.controller())
            && self.get_power(PowerType::OperateController).is_some()
            && self.home.is_power_enabled(&PowerType::OperateController)
        {
            if self.creep.store().get_used_capacity(Some(ResourceType::Ops)) < 200 {
                if let Some(storage) = self.home.storage()
                    && storage.store().get_used_capacity(Some(ResourceType::Ops)) >= 200
                {
                    if self.creep.pos().is_near_to(storage.pos()) {
                        let _ = self.creep.withdraw(storage, ResourceType::Ops, None);
                        None
                    } else {
                        build_goal(storage.pos(), CLOSE_RANGE_ACTION, None)
                    }
                } else {
                    warn!("room: {} resource ops not enough!!", self.home.name());
                    None
                }
            } else {
                debug!("creep full: {}", self.creep.name());
                if self.creep.pos().get_range_to(self.home.controller().pos()) <= LONG_RANGE_ACTION
                {
                    let res = self
                        .creep
                        .use_power(PowerType::OperateController, Some(self.home.controller()));
                    debug!("creep {} operate controller res: {:?}", self.creep.name(), res);
                    match res {
                        Ok(()) => {}
                        Err(err) => {
                            error!("use power error: {:?}", err);
                        }
                    }
                    None
                } else {
                    build_goal(self.home.controller().pos(), LONG_RANGE_ACTION, None)
                }
            }
        } else {
            None
            // go_to_workplace(creep, home);
        }
        // None
    }

    fn renew(&self) -> Option<MovementGoal> {
        if let Some(power_spawn) = self.home.power_spawn() {
            if self.creep.pos().is_near_to(power_spawn.pos()) {
                let _ = self.creep.renew(power_spawn);
                None
            } else {
                build_goal(power_spawn.pos(), CLOSE_RANGE_ACTION, None)
            }
        } else {
            warn!("power_creep: {} no powerspawn found for renew!!", self.creep.name());
            None
        }
    }

    fn is_power_available(&self, power_type: PowerType) -> bool {
        self.creep.powers().get(power_type).is_some_and(|power| power.cooldown() == 0)
    }

    fn get_power(&self, power_type: PowerType) -> Option<PowerInfo> {
        self.creep
            .powers()
            .get(power_type)
            .and_then(|p| if p.cooldown() == 0 { Some(p) } else { None })
    }

    pub fn move_to_goal(self, mut goal: Option<MovementGoal>, movement: &mut Movement) {
        let position = self.pos();
        //creep is not resting and is able to move
        if let Some(mut movement_goal) = goal.take() {
            if movement_goal.is_goal_met(position) {
                // goal is met! unset the path_state if there is one and idle
                movement.idle(position, self.creep.into());
                self.memory.path_state = None;
            } else {
                let new_path_state = if let Some(mut current_path) = self.memory.path_state.take() {
                    // first call the function that updates the current position
                    // (or the stuck count if we didn't move)
                    if current_path.check_if_moved_and_update_pos(position) {
                        PathState::try_new(position, movement_goal, movement.get_find_route_options())
                    } else if current_path.stuck_threshold_exceed() {
                        debug!("{}, is last step, progress: {}, path.len: {}, stuck.count: {}", self.name(), current_path.path_progress, current_path.path.len(), current_path.stuck_count);
                        movement_goal.avoid_creeps = true;
                        PathState::try_new(position, movement_goal, movement.get_find_route_options())
                    } else if movement_goal.pos != current_path.goal.pos || movement_goal.range < current_path.goal.range {
                        //if goal pos is changed -> find new path
                        PathState::try_new(position, movement_goal, movement.get_find_route_options())
                    } else if movement_goal.repath_needed(&current_path.goal) {
                        if let Some(new_path) = PathState::try_new(position, movement_goal, movement.get_find_route_options()) {
                            //todo prefer longest way if enemies nearby? many enemies? boosted?
                            if new_path.path.len() + 5 < current_path.path.len() {
                                debug!("{} from: {}, new path + 5: {} shorter then prev: {}, new path: {:?}", self.name(), position, new_path.path.len(), current_path.path.len(), new_path);
                                Some(new_path)
                            } else {
                                Some(current_path)
                            }
                        } else {
                            Some(current_path)
                        }
                    } else {
                        //if nothing is changed -> use current path
                        Some(current_path)
                    }
                } else {
                    PathState::try_new(position, movement_goal, movement.get_find_route_options())
                }
                .and_then(|path_state| movement.move_creep(self.creep.into(), path_state));

                self.memory.path_state = new_path_state;
            }
        } else {
            // no goal, mark as idle!
            movement.idle(position, self.creep.into());
        }
    }
}

fn controller_without_effect(controller: &StructureController) -> bool {
    !controller.effects().into_iter().any(|effect: Effect| match effect.effect() {
        EffectType::PowerEffect(p) => matches!(p, PowerType::OperateController),
        _ => false,
    })
}

fn enable_controller(creep: &PowerCreep, controller: &StructureController) -> Option<MovementGoal> {
    if creep.pos().is_near_to(controller.pos()) {
        let _ = creep.enable_room(controller);
        None
    } else {
        build_goal(controller.pos(), CLOSE_RANGE_ACTION, None)
    }
}

fn build_goal(
    pos: Position,
    range: u32,
    danger_zones: Option<(RoomName, Vec<RoomXY>)>,
) -> Option<MovementGoal> {
    Some(
        MovementGoalBuilder::new(pos)
            .range(range)
            .profile(MovementProfile::SwampFiveToOne)
            .avoid_creeps(false)
            .danger_zones(danger_zones)
            .build(),
    )
}
