use std::collections::HashMap;
use agent::Agent;
use transport_mode::TransportMode;
use journey_type::JourneyType;
use subculture::Subculture;
use neighbourhood::Neighbourhood;
use std::rc::Rc;
use std::cell::RefCell;
use itertools::Itertools;

/// Counts the number of agents who take an active mode
/// * agents: The agents to count from
/// * Returns: The number of agents who's current_mode is either Walk or Cycle
pub fn count_active_mode(agents: &[Rc<RefCell<Agent>>]) -> usize {
    agents
        .iter()
        .filter(|&a| a.borrow().current_mode == TransportMode::Walk
            || a.borrow().current_mode == TransportMode::Cycle)
        .count()
}

/// Counts the number of agents who take an active mode counter to their inactive norm
/// * agents: The agents to count from
/// * Returns: The number of agent's who's current mode is Walk or Cycle but their norm is Car or PublicTransport
pub fn count_active_mode_counter_to_inactive_norm(agents: &[Rc<RefCell<Agent>>]) -> usize {
    agents
        .iter()
        .filter(|&a|
            (a.borrow().current_mode == TransportMode::Walk
                || a.borrow().current_mode == TransportMode::Cycle)
            && (a.borrow().norm != TransportMode::Walk && a.borrow().norm != TransportMode::Cycle))
        .count()
}

/// Counts the number of agents who take an inactive mode counter to their active norm
/// * agents: The agents to count from
/// * Returns: The number of agent's who's current mode is Car or Public Transport but their norm is Walk or Cycle
pub fn count_inactive_mode_counter_to_active_norm(agents: &[Rc<RefCell<Agent>>]) -> usize {
    agents
        .iter()
        .filter(|&a|
            (a.borrow().current_mode != TransportMode::Walk
                && a.borrow().current_mode != TransportMode::Cycle)
            && (a.borrow().norm == TransportMode::Walk || a.borrow().norm == TransportMode::Cycle))
        .count()
}

/// Counts the number of agents who have an active norm
/// * agents: The agents to count from
/// * Returns: The number of agent's who's norm is Walk or Cycle
pub fn count_active_norm(agents: &[Rc<RefCell<Agent>>]) -> usize {
    agents
        .iter()
        .filter(|&a|
            a.borrow().norm == TransportMode::Walk || a.borrow().norm == TransportMode::Cycle)
        .count()
}

/// Counts the number of agents who take an active mode grouped by commute length
/// * agents: The agents to count from
/// * Returns: A Map: JourneyType -> The number of agent's who's current mode is Walk or Cycle 
pub fn count_active_mode_by_commute_length(agents: &[Rc<RefCell<Agent>>]) -> HashMap<JourneyType, usize> {
    agents
        .iter()
        .map(|agent| (agent.borrow().commute_length, Rc::clone(agent)))
        .into_group_map()
        .into_iter()
        .map(|(journey_type, grouped_agents)| (journey_type, count_active_mode(&grouped_agents)))
        .collect()
}

/// Counts the number of agents who take an active mode grouped by Subculture
/// * agents: The agents to count from
/// * Returns: A Map: Subculture -> The number of agent's who's current mode is Walk or Cycle 
pub fn count_active_mode_by_subculture (agents: &[Rc<RefCell<Agent>>]) -> HashMap<Rc<Subculture>, usize> {
    agents
        .iter()
        .map(|agent| (Rc::clone(&agent.borrow().subculture), Rc::clone(agent)))
        .into_group_map()
        .into_iter()
        .map(|(subculture, grouped_agents)| (subculture, count_active_mode(&grouped_agents)))
        .collect()
}

/// Counts the number of agents who take an active mode grouped by neighbourhood
/// * neighbourhoods: The neighbourhoods to count from
/// * Returns: A Map: Neighbourhood -> The number of agent's who's current mode is Walk or Cycle 
pub fn count_active_mode_by_neighbourhood (neighbourhoods: &[Rc<Neighbourhood>]) -> HashMap<Rc<Neighbourhood>, usize> {
    neighbourhoods
        .iter()
        .map(|neighbourhood| (Rc::clone(neighbourhood), count_active_mode(&neighbourhood.residents.borrow())))
        .collect()
}
