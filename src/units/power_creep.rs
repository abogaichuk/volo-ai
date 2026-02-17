use std::collections::HashMap;

use log::{debug, error};
use screeps::action_error_codes::{EnableRoomErrorCode, RenewErrorCode, TransferErrorCode, UsePowerErrorCode, WithdrawErrorCode};
use screeps::{
    Creep, HasPosition, Mineral, Position, PowerCreep, PowerInfo, PowerType, ResourceType, RoomName, RoomObject, RoomObjectProperties, SharedCreepProperties, Source, StructureController, StructureFactory, StructurePowerSpawn, StructureRampart, StructureSpawn, StructureStorage, StructureTower, Transferable, Withdrawable, game
};
use serde::{Deserialize, Serialize};

use crate::movement::{Movement, MovementGoal, MovementGoalBuilder, MovementProfile, PathState};
use crate::rooms::shelter::Shelter;
use crate::units::actions::{common_actions, end_of_chain, fortify, operate_controller, operate_factory, operate_mineral, operate_source, operate_spawn, operate_storage, operate_tower, transfer, withdraw};
use crate::movement::walker::get_danger_zones;
use crate::utils::commons;
use crate::utils::constants::TOWER_ATTACK_RANGE;


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
        let Some(pc) = p_creeps.remove(name) else { continue };

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
            memory.home = get_home(&pc, homes).map(super::super::rooms::shelter::Shelter::name);
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
    pub fn name(&self) -> String {
        self.creep.name()
    }

    pub fn pos(&self) -> Position {
        self.creep.pos()
    }

    pub fn ticks_to_live(&self) -> Option<u32> {
        self.creep.ticks_to_live()
    }

    pub fn used_capacity(&self, res: Option<ResourceType>) -> u32 {
        self.creep.store().get_used_capacity(res)
    }

    pub fn capacity(&self) -> u32 {
        self.creep.store().get_capacity(None)
    }

    pub fn is_power_enabled(&self, power: PowerType) -> bool {
        self.home.is_power_enabled(power)
    }

    pub fn home_name(&self) -> RoomName {
        self.home.name()
    }

    pub fn home_mineral(&self) -> &Mineral {
        self.home.mineral()
    }

    pub fn home_sources(&self) -> &[Source] {
        self.home.sources()
    }

    pub fn home_storage(&self) -> Option<&StructureStorage> {
        self.home.storage()
    }

    pub fn home_factory(&self) -> Option<&StructureFactory> {
        self.home.factory()
    }

    pub fn home_power_spawn(&self) -> Option<&StructurePowerSpawn> {
        self.home.power_spawn()
    }

    pub fn home_controller(&self) -> &StructureController {
        self.home.controller()
    }

    pub fn home_spawns(&self) -> &[StructureSpawn] {
        self.home.spawns()
    }

    pub fn home_towers(&self) -> &[StructureTower] {
        self.home.towers()
    }

    pub fn is_home_invaded(&self) -> bool {
        self.home.invasion()
    }

    pub fn get_hostiles_at_home(&self) -> &[Creep] {
        self.home.get_hostiles(None)
    }

    pub fn home_lowest_perimeter(&self) -> Option<&StructureRampart> {
        self.home.lowest_perimetr_hits()
    }

    pub fn withdraw<T>(
        &self,
        target: &T,
        ty: ResourceType,
        amount: Option<u32>,) -> Result<(), WithdrawErrorCode>
    where
        T: Withdrawable + ?Sized
    {
        self.creep.withdraw(target, ty, amount)
    }

    pub fn transfer<T>(
        &self,
        target: &T,
        ty: ResourceType,
        amount: Option<u32>,
    ) -> Result<(), TransferErrorCode>
    where
        T: Transferable + ?Sized,
    {
        self.creep.transfer(target, ty, amount)
    }

    pub fn use_power(
        &self,
        power: PowerType,
        target: Option<&RoomObject>,
    ) -> Result<(), UsePowerErrorCode> {
        self.creep.use_power(power, target)
    }

    pub fn enable_room(&self, target: &StructureController) -> Result<(), EnableRoomErrorCode> {
        self.creep.enable_room(target)
    }

    pub fn renew(&self, target: &RoomObject) -> Result<(), RenewErrorCode> {
        self.creep.renew(target)
    }

    pub fn run_unit(&mut self) -> Option<MovementGoal> {

        let hostiles = self.get_hostiles_at_home();

        let actions = if self.is_home_invaded() {
            common_actions(withdraw(10, operate_tower(fortify(end_of_chain()))))
        } else if hostiles.iter()
            .find(|hostile| commons::remoted_from_edge(hostile.pos(), TOWER_ATTACK_RANGE)).is_some()
        {
            common_actions(
                withdraw(
                    100,
                    operate_storage(
                        operate_tower(
                            operate_source(
                                operate_mineral(end_of_chain()))))))
        } else {
            common_actions(operate_source(
                withdraw(
                    200, 
                    operate_storage(
                        operate_mineral(
                            operate_spawn(
                                operate_factory(
                                    operate_controller(
                                        transfer(end_of_chain())))))))))
        };

        actions(self)
            .map(|(target, range)| {
                let danger_zones = get_danger_zones(target.room_name(), hostiles);
                MovementGoalBuilder::new(target)
                    .range(range)
                    .profile(MovementProfile::SwampFiveToOne)
                    .avoid_creeps(false)
                    .danger_zones(danger_zones)
                    .build()
            })

    }

    pub fn is_power_available(&self, power_type: PowerType) -> bool {
        self.creep.powers().get(power_type).is_some_and(|power| power.cooldown() == 0)
    }

    pub fn get_power(&self, power_type: PowerType) -> Option<PowerInfo> {
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
