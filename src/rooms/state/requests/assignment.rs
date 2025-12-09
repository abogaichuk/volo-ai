use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use screeps::game;
use serde::{Deserialize, Serialize};

use super::{Meta, RequestError, Status};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Assignment {
    /// No doer at all, executed by the room manager or room structure
    None,
    /// At most 1 doer
    Single(Option<String>),
    /// Many doers in one pool
    Multi(HashSet<String>),
    /// Sequential squads
    Squads(Vec<Squad>),
}

impl Assignment {
    //creep name or squad name
    pub(super) fn has_member(&self, name: &str) -> bool {
        match self {
            Assignment::None => false,
            Assignment::Single(slot) => slot.as_ref().is_some_and(|doer| doer == name),
            Assignment::Multi(set) => set.contains(name),
            Assignment::Squads(squads) => squads.iter().any(|s| s.id == *name),
        }
    }

    pub fn new_squad(&mut self, id_part: String, meta: &mut Meta) -> Option<String> {
        match self {
            Assignment::Squads(squads) => {
                let squad_index = squads.len() + 1;
                let squad_id = format!("{}_{}", id_part, squad_index);

                let squad = Squad { id: squad_id.clone(), members: HashSet::new() };

                squads.push(squad);
                meta.status = Status::InProgress;
                meta.updated_at = game::time();
                Some(squad_id)
            }
            _ => None,
        }
    }

    pub fn squads_members(&self, squad_id: &str) -> Option<HashSet<String>> {
        match self {
            Assignment::Squads(squads) => {
                squads.iter().find(|squad| squad.id == *squad_id).map(|squad| squad.members.clone())
            }
            _ => None,
        }
    }

    pub fn has_any_members(&self) -> bool {
        match self {
            Assignment::None => false,
            Assignment::Single(slot) => slot.is_some(),
            Assignment::Multi(set) => !set.is_empty(),
            Assignment::Squads(squads) => squads.iter().any(|s| !s.members.is_empty()),
        }
    }

    pub fn has_alive_members(&self) -> bool {
        match self {
            Assignment::None => false,
            Assignment::Single(slot) => {
                slot.as_ref().is_some_and(|doer| game::creeps().get(doer.to_string()).is_some())
            }
            Assignment::Multi(set) => {
                set.iter().any(|doer| game::creeps().get(doer.to_string()).is_some())
            }
            Assignment::Squads(squads) => squads.iter().any(|s| {
                s.members.iter().any(|member| game::creeps().get(member.clone()).is_some())
            }),
        }
    }

    pub fn drop(&mut self, doer: String, squad_id: Option<&str>) -> Result<(), RequestError> {
        match self {
            Assignment::None => Ok(()),
            Assignment::Single(slot) => match slot {
                None => Ok(()),
                Some(by) => {
                    if *by == doer {
                        Ok(())
                    } else {
                        Err(RequestError::InvalidAssignment(format!(
                            "doer {} is not working on Assignment::Single({})",
                            doer, by
                        )))
                    }
                }
            },
            Assignment::Multi(set) => {
                if set.remove(&doer) {
                    Ok(())
                } else {
                    Err(RequestError::InvalidAssignment(format!(
                        "doer {} is not working on Assignment::Multi({:?})",
                        doer, set
                    )))
                }
            }
            Assignment::Squads(squads) => {
                let id = match squad_id {
                    Some(id) => id,
                    None => {
                        return Err(RequestError::InvalidAssignment(
                            "can't drop None from Assignment::Squad".to_string(),
                        ));
                    }
                };

                if let Some(index) = squads.iter().position(|s| s.id == id) {
                    let mut squad = squads.remove(index);
                    squad.members.remove(&doer);

                    if !squad.members.is_empty() {
                        squads.insert(index, squad);
                    }
                    Ok(())
                } else {
                    Err(RequestError::InvalidAssignment(format!("squad id {} not found!", id)))
                }
            }
        }
    }

    pub fn try_join(
        &mut self,
        doer: Option<String>,
        squad_id: Option<&str>,
    ) -> Result<(), RequestError> {
        match self {
            Assignment::None => {
                if let Some(d) = doer {
                    Err(RequestError::InvalidAssignment(format!(
                        "doer {} can't be assigned to None",
                        d
                    )))
                } else if let Some(s) = squad_id {
                    Err(RequestError::InvalidAssignment(format!(
                        "squad_id {} can't be assigned to None",
                        s
                    )))
                } else {
                    Ok(())
                }
            }
            Assignment::Single(slot) => match doer {
                None => Err(RequestError::InvalidAssignment(
                    "None can't be assigned to Assignment::Single".to_string(),
                )),
                Some(d) => match slot {
                    None => {
                        *slot = Some(d);
                        Ok(())
                    }
                    Some(by) => {
                        if *by == d {
                            Ok(())
                        } else {
                            Err(RequestError::AssignmentBusy(
                                d,
                                Assignment::Single(Some(by.clone())),
                            ))
                        }
                    }
                },
            },
            Assignment::Multi(set) => {
                if let Some(d) = doer {
                    set.insert(d);
                    Ok(())
                } else {
                    Err(RequestError::InvalidAssignment(
                        "None can't be assigned to Assignment::Multi".to_string(),
                    ))
                }
            }
            Assignment::Squads(squads) => {
                let id = match squad_id {
                    Some(id) => id,
                    None => {
                        return Err(RequestError::InvalidAssignment(
                            "None can't be assigned to Assignment::Squad".to_string(),
                        ));
                    }
                };

                if let Some(squad) = squads.iter_mut().find(|s| s.id == id) {
                    match doer {
                        Some(d) => {
                            squad.members.insert(d);
                            Ok(())
                        }
                        None => Err(RequestError::InvalidAssignment(
                            "None can't be assigned to Assignment::Squad".to_string(),
                        )),
                    }
                } else {
                    Err(RequestError::InvalidSquadId(id.to_string()))
                }
            }
        }
    }

    pub fn remove_doer(&mut self, doer: &str) -> bool {
        match self {
            Assignment::None => false,
            Assignment::Single(slot) => {
                if matches!(slot.as_deref(), Some(name) if name == doer) {
                    *slot = None;
                    true
                } else {
                    false
                }
            }
            Assignment::Multi(set) => set.remove(doer),
            Assignment::Squads(squads) => {
                let mut removed = false;
                for squad in squads {
                    if squad.members.remove(doer) {
                        removed = true;
                    }
                }
                removed
            }
        }
    }
}

impl Display for Assignment {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Assignment::None => write!(f, "None"),
            Assignment::Single(name) => write!(f, "Single({:?})", name),
            Assignment::Multi(names) => write!(f, "Multi({:?})", names),
            Assignment::Squads(squads) => write!(f, "Squads({:?})", squads),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Squad {
    pub id: String,
    pub members: HashSet<String>,
}
