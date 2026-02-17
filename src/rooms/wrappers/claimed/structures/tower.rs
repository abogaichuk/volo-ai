use log::info;
use screeps::objects::Creep;
use screeps::prelude::*;
use screeps::{INVADER_USERNAME, Part, Position, PowerCreep};

use crate::commons::full_boosted;
use crate::rooms::wrappers::claimed::Claimed;
use crate::utils::commons;
use crate::utils::constants::TOWER_ATTACK_RANGE;

impl Claimed {
    pub(crate) fn run_towers(&self) {
        let perimetr: Vec<Position> =
            self.ramparts.perimeter().map(screeps::HasPosition::pos).collect();

        let (hostiles, invanders): (Vec<_>, _) =
            self.hostiles.iter().partition(|enemy| enemy.owner().username() != INVADER_USERNAME);

        if !hostiles.is_empty() {
            let (boosted, unboosted): (Vec<&&Creep>, _) =
                hostiles.iter().partition(|hostile| full_boosted(hostile));

            if !boosted.is_empty() {
                let healers_dont_have = [Part::Attack, Part::RangedAttack, Part::Work];
                let (healers, another): (Vec<&&Creep>, _) = boosted.iter().partition(|hostile| {
                    hostile
                        .body()
                        .iter()
                        .all(|body_part| !healers_dont_have.contains(&body_part.part()))
                });

                let (_can_heal, without_heal): (Vec<&&Creep>, _) =
                    another.iter().partition(|hostile| {
                        hostile.body().iter().any(|body_part| body_part.part() == Part::Heal)
                    });

                if let Some(target) = without_heal.iter().find(|hostile| {
                    self.towers.iter().any(|tower| tower.pos().get_range_to(hostile.pos()) <= 5)
                        && healers.iter().all(|healer| healer.pos().get_range_to(hostile.pos()) > 2)
                }) {
                    self.mass_attack(target);
                } else if let Some(injured) =
                    boosted.iter().find(|hostile| hostile.hits() < hostile.hits_max())
                {
                    self.mass_attack(injured);
                } else if let Some(closest) =
                    boosted.iter().find(|enemy| is_near_to(enemy, &perimetr))
                    && self.controller.level() >= 6
                {
                    self.mass_attack(closest);
                } else {
                    self.distributed_heal(find_all_injured(&self.my_creeps, &self.my_pcreeps));
                }
            } else if !unboosted.is_empty() {
                if let Some(injured) =
                    unboosted.iter().find(|hostile| hostile.hits() < hostile.hits_max())
                {
                    self.mass_attack(injured);
                } else if let Some(scout) =
                    unboosted.iter().find(|hostile| hostile.body().len() < 2)
                    && let Some(tower) = get_closest(scout, self.towers.iter())
                {
                    let _ = tower.attack::<Creep>(scout);
                } else if let Some(in_range) =
                    unboosted.iter().find(|hostile| commons::remoted_from_edge(hostile.pos(), TOWER_ATTACK_RANGE))
                {
                    self.mass_attack(in_range);
                } else {
                    self.distributed_heal(find_all_injured(&self.my_creeps, &self.my_pcreeps));
                }
            } else {
                self.distributed_heal(find_all_injured(&self.my_creeps, &self.my_pcreeps));
            }
        } else if let Some(invander) = invanders.first() {
            info!("{} mass attack invander", self.get_name());
            self.mass_attack(invander);
        } else {
            self.distributed_heal(find_all_injured(&self.my_creeps, &self.my_pcreeps));
        }
    }

    fn distributed_heal<'a, I>(&self, injured_creeps: I)
    where
        I: Iterator<Item = &'a dyn Healable>,
    {
        let mut towers = self.towers.clone();
        injured_creeps.for_each(|creep| {
            if !towers.is_empty() {
                towers.sort_by_key(|tower| tower.pos().get_range_to(creep.pos()));
                let closest_tower = towers.swap_remove(0);
                let _ = closest_tower.heal(creep);
            }
        });
    }

    fn mass_attack(&self, target: &Creep) {
        self.towers.iter().for_each(|tower| {
            let _ = tower.attack(target);
        });
    }
}

fn get_closest<'a, T>(to: &dyn HasPosition, iterator: impl Iterator<Item = &'a T>) -> Option<&'a T>
where
    T: HasPosition,
{
    iterator.fold(None, |acc, elem| {
        if let Some(obj) = acc {
            if elem.pos().get_range_to(to.pos()) < obj.pos().get_range_to(to.pos()) {
                Some(elem)
            } else {
                Some(obj)
            }
        } else {
            Some(elem)
        }
    })
}

fn is_near_to<T>(obj: &dyn HasPosition, positions: &[T]) -> bool
where
    T: HasPosition,
{
    positions.iter().any(|pos| pos.pos().is_near_to(obj.pos()))
}

fn find_all_injured<'a>(
    creeps: &'a [Creep],
    p_creeps: &'a [PowerCreep],
) -> impl Iterator<Item = &'a dyn Healable> {
    creeps
        .iter()
        .filter(|creep| creep.hits() < creep.hits_max())
        .map(|creep| creep as &dyn Healable)
        .chain(p_creeps.iter().filter(|pc| pc.hits() < pc.hits_max()).map(|pc| pc as &dyn Healable))
}
