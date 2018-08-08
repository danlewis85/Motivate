use rand;
use std::collections::HashMap;
use std::io::Write;
use std::io::Read;
use rand::distributions;
use rand::distributions::Distribution;
use rand::thread_rng;
use rand::seq::sample_slice_ref;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use transport_mode::TransportMode;
use journey_type::JourneyType;
use neighbourhood::Neighbourhood;
use subculture::Subculture;
use scenario::Scenario;
use agent::Agent;
use gaussian;
use serde_yaml;

/// Loads unlinked agents from a YAML file
/// * file: The file to load from
/// * subcultures: The subcultures in the scenario
/// * neighbourhoods: The neighbourhoods in the scenario
/// * Returns: The loaded agents
pub fn load_unlinked_agents_from_file(mut file: File, subcultures: &[Rc<Subculture>], neighbourhoods: &[Rc<Neighbourhood>]) -> Vec<Rc<RefCell<Agent>>> {
        info!("Loading parameters from file");
        let mut file_contents = String::new();

        file.read_to_string(&mut file_contents)
            .expect("There was an error reading the file");

        let mut residents: Vec<Rc<RefCell<Agent>>> = serde_yaml::from_slice(file_contents.as_bytes())
            .expect("There was an error parsing the file");

        let subcultures_kvp: HashMap<String, Rc<Subculture>> = subcultures
            .iter()
            .map(|subculture| (subculture.id.clone(), Rc::clone(subculture)))
            .collect();

        let neighbourhoods_kvp: HashMap<String, Rc<Neighbourhood>> = neighbourhoods
            .iter()
            .map(|neighbourhood| (neighbourhood.id.clone(), Rc::clone(neighbourhood)))
            .collect();

        for agent in residents.iter_mut() {
            let subculture = Rc::clone(subcultures_kvp.get(&agent.borrow().subculture_id).expect("Subculture not found"));
            agent.borrow_mut().subculture = subculture;
            let neighbourhood = Rc::clone(neighbourhoods_kvp.get(&agent.borrow().neighbourhood_id).expect("Agent not found"));
            neighbourhood.residents.borrow_mut().push(Rc::clone(agent));
            agent.borrow_mut().neighbourhood = neighbourhood;
        }

        residents
}

/// Saves the agents to a file
/// * file: The file to save them to
/// * agents: The agents to save
fn save_agents(mut file: File, agents: &[Rc<RefCell<Agent>>]) {
    let agents_string = serde_yaml::to_string(agents).unwrap();
    file.write_all(agents_string.as_bytes()).unwrap();
}

/// Generates the (unlinked) agents and saves them to a file
/// * file: The file to save them to
/// * scenario: The scenario of the simulation
/// * social_connectivity: How connected the agent is to its social network
/// * subculture_connectivity: How connected the agent is to its subculture
/// * neighbourhood_connectivity: How connected the agent is to its neighbourhood
/// * days_in_habit_average: How many days should be used in the habit average
/// * number_of_people: The number of agents to generate
/// * distributions: JourneyType distributions, used in gmm
/// * Returns: The created agents
pub fn generate_and_save_agents(
    file: File,
    scenario: &Scenario,
    social_connectivity: f32,
    subculture_connectivity: f32,
    neighbourhood_connectivity: f32,
    days_in_habit_average: u32,
    number_of_people: u32,
    distributions: Vec<(f64, f64, f64)>
    ) -> Vec<Rc<RefCell<Agent>>>
{
    let agents = generate_unlinked_agents(
        scenario, 
        social_connectivity, 
        subculture_connectivity, 
        neighbourhood_connectivity, 
        days_in_habit_average, 
        number_of_people, 
        distributions);
    
    save_agents(file, &agents);

    agents
}

/// Create the agents  
/// Returns a set of Rc pointers, storing RefCells of Agents  
/// Rc are reference counter pointers, that are store immutable data,  
/// RefCells are immutable but have mutable contents  
/// Meaning that Agents are mutable  
/// See [here](https://doc.rust-lang.org/book/second-edition/ch15-05-interior-mutability.html) for more details
/// * scenario: The scenario of the simulation
/// * social_connectivity: How connected the agent is to its social network
/// * subculture_connectivity: How connected the agent is to its subculture
/// * neighbourhood_connectivity: How connected the agent is to its neighbourhood
/// * days_in_habit_average: How many days should be used in the habit average
/// * number_of_people: The number of agents to generate
/// * distributions: JourneyType distributions, used in gmm
/// * Returns: The created agents
fn generate_unlinked_agents(scenario: &Scenario,
          social_connectivity: f32,
          subculture_connectivity: f32,
          neighbourhood_connectivity: f32,
          days_in_habit_average: u32,
          number_of_people: u32,
          distributions: Vec<(f64, f64, f64)>) -> Vec<Rc<RefCell<Agent>>> {
    // Create an empty vec to store agents
    let mut residents = Vec::new();
    // Create self.number_of_people unlinked agents
    for _ in 0..number_of_people {
        let agent = create_unlinked_agent(scenario, social_connectivity,
            subculture_connectivity, neighbourhood_connectivity, days_in_habit_average);

        let rc_agent = Rc::new(RefCell::new(agent));
        
        {
            // Add rc_agent to its neighbourhood's residents
            // This needs to be in a different scope
            let agent_ref = &rc_agent.borrow();

            agent_ref
                .neighbourhood
                .residents
                .borrow_mut()
                .push(Rc::clone(&rc_agent));
        }

        residents.push(rc_agent);
    }

    // This needs to be in a different scope, so that residents can be borrowed as immutable
    // later on, when called at link_agents_from_predefined_network. Rust allows multiple
    // immutable borrows, or one mutable borrow at any given time, if this is not in a different
    // scope then &residents and &mut residents are in scope at the same time.
    {
        // Give people cars
        let mut rng = thread_rng();
        let sample = sample_slice_ref(&mut rng, &residents, scenario.number_of_cars as usize);
        sample
            .iter()
            .for_each(|agent| agent.borrow_mut().owns_car = true);
    }
    {
        // Give people bikes
        let mut rng = thread_rng();
        let sample = sample_slice_ref(&mut rng, &residents, scenario.number_of_cars as usize);
        sample
            .iter()
            .for_each(|agent| agent.borrow_mut().owns_bike = true);
    }

    // Get random commute distances
    let commute_distances: Vec<f64> = gaussian::get_samples_from_gmm(number_of_people as usize, distributions)
        .into_iter()
        .map(|x| x.abs())
        .collect();

    // Assign commute distances
    commute_distances
        .iter()
        .zip(residents.iter())
        .for_each(|(distance, agent)| {
            let agent_ref = &mut agent.borrow_mut();

            agent_ref.commute_length_continuous = *distance;

            // Assign categorical distance
            if *distance < 4241.0 {
                agent_ref.commute_length = JourneyType::LocalCommute
            } else if *distance < 19457.0 {
                agent_ref.commute_length = JourneyType::CityCommute
            } else {
                agent_ref.commute_length = JourneyType::DistantCommute
            }
        });

    // For each agent, choose an initial mode
    for agent in residents.iter() {
        let mut borrowed_agent = agent.borrow_mut();
        let new_mode = choose_initial_norm_and_habit(
            borrowed_agent.owns_car, 
            borrowed_agent.owns_bike);
        
        borrowed_agent.current_mode = new_mode;
        borrowed_agent.habit = hashmap!{new_mode => 1.0f32};
        borrowed_agent.norm = new_mode;
    }

    // Return the created agents
    residents
}

/// Create an unlinked agent, that does not own a bike or a car, without a current mode, and without a commute length
/// * scenario: The scenario of the simulation
/// * social_connectivity: How connected the agent is to its social network
/// * subculture_connectivity: How connected the agent is to its subculture
/// * neighbourhood_connectivity: How connected the agent is to its neighbourhood
/// * days_in_habit_average: How many days should be used in the habit average
/// * Returns: The created agent
fn create_unlinked_agent(scenario: &Scenario,
                         social_connectivity: f32,
                         subculture_connectivity: f32,
                         neighbourhood_connectivity: f32,
                         days_in_habit_average: u32) -> Agent {
    // Choose a subculture and, neighbourhood
    let subculture = choose_subculture(scenario);
    let neighbourhood = choose_neighbourhood(scenario);

    // Weather sensitivity is currently fixed
    let weather_sensitivity = rand::random::<f32>();
    // TODO: Should consistency be used
    let consistency = 1.0f32;

    // Use a placeholder transport mode
    let current_mode: TransportMode = TransportMode::PublicTransport;
    let norm = current_mode;
    let last_mode = current_mode;

    // Create and return the agent
    Agent {
        subculture_id: subculture.id.clone(),
        subculture,
        neighbourhood_id: neighbourhood.id.clone(),
        neighbourhood,
        commute_length: JourneyType::LocalCommute,
        commute_length_continuous: 0.0,
        weather_sensitivity,
        consistency,
        social_connectivity: social_connectivity,
        subculture_connectivity: subculture_connectivity,
        neighbourhood_connectivity: neighbourhood_connectivity,
        average_weight: 2.0 / (days_in_habit_average as f32 + 1.0),
        habit: hashmap!{current_mode => 1.0f32},
        current_mode,
        last_mode,
        norm,
        owns_bike: false,
        owns_car: false,
        social_network: Vec::new(),
        neighbours: Vec::new(),
    }
}

/// Choose a random neighbourhood, equal chance of each
/// * scenario: The scenario of the simulation
/// * Returns: The chosen neighbourhood
fn choose_neighbourhood(scenario: &Scenario) -> Rc<Neighbourhood> {
    let mut weighted: Vec<distributions::Weighted<Rc<Neighbourhood>>> = scenario.neighbourhoods
        .iter()
        .map(|s: &Rc<Neighbourhood>| distributions::Weighted {weight: 1, item: Rc::clone(s)})
        .collect();
    let weighted_choice = distributions::WeightedChoice::new(&mut weighted);
    weighted_choice.sample(&mut thread_rng())
}

/// Choose a random subculture, equal chance of each
/// * scenario: The scenario of the simulation
/// * Returns: The chosen subculture
fn choose_subculture(scenario: &Scenario) -> Rc<Subculture> {
    let mut weighted: Vec<distributions::Weighted<Rc<Subculture>>> = scenario.subcultures
        .iter()
        .map(|s: &Rc<Subculture>| distributions::Weighted {weight: 1, item: Rc::clone(s)})
        .collect();
    let weighted_choice = distributions::WeightedChoice::new(&mut weighted);
    weighted_choice.sample(&mut thread_rng())
}

/// Choose an initial norm and habit
/// * owns_car: whether the agent owns a car
/// * owns_bike: whether the agent owns a bike
/// * Returns: The chosen transport mode
fn choose_initial_norm_and_habit(owns_car: bool, owns_bike: bool) -> TransportMode {
    if owns_car && owns_bike {
        let randfloat = rand::random::<f64>();
        if randfloat < 0.4 {
            TransportMode::Car
        } else if randfloat < 0.7 {
            TransportMode::Cycle
        } else if randfloat < 0.85 {
            TransportMode::Walk
        } else {
            TransportMode::PublicTransport
        }
    } else if owns_car {
        let randfloat = rand::random::<f64>();
        if randfloat < 0.57 {
            TransportMode::Car
        }
        else if randfloat < 0.79 {
            TransportMode::Walk
        } else {
            TransportMode::PublicTransport
        }
    } else if owns_bike {
        let randfloat = rand::random::<f64>();
        if randfloat < 0.5 {
            TransportMode::Cycle
        } else if randfloat < 0.75 {
            TransportMode::Walk
        } else {
            TransportMode::PublicTransport
        }
    } else {
        let randfloat = rand::random::<f64>();
        if randfloat < 0.5 {
            TransportMode::Walk
        } else {
            TransportMode::PublicTransport
        }
    }
}