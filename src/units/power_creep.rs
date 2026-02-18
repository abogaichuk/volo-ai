use std::collections::HashMap;

use log::error;
use screeps::action_error_codes::{EnableRoomErrorCode, RenewErrorCode, TransferErrorCode, UsePowerErrorCode, WithdrawErrorCode};
use screeps::{
    Creep, HasPosition, Mineral, Position, PowerCreep, PowerInfo, PowerType, ResourceType, RoomName,
    RoomObject, RoomObjectProperties, SharedCreepProperties, Source, StructureController, StructureFactory,
    StructurePowerSpawn, StructureRampart, StructureSpawn, StructureStorage, StructureTower, Transferable,
    Withdrawable, game
};
use serde::{Deserialize, Serialize};

use crate::movement::{Movement, MovementGoal, MovementGoalBuilder, MovementProfile, PathState, walker::get_danger_zones};
use crate::rooms::shelter::Shelter;
use crate::units::{
    move_to_goal_common,
    actions::{
        common_actions, end_of_chain, fortify, operate_controller, operate_factory, operate_mineral,
        operate_source, operate_spawn, operate_storage, operate_tower, transfer, withdraw
    }};
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
        let name = self.name();
        let position = self.pos();
        let unit = self.creep.into();

        move_to_goal_common(
            name.as_str(),
            position,
            unit,
            goal.take(),
            movement,
            &mut self.memory.path_state,
            true,
        );
    }
}
