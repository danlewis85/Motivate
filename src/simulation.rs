extern crate rand;

use std;
use std::collections::HashMap;
use std::io::Write;
use std::io::BufWriter;
use itertools::Itertools;
use std::time::SystemTime;
use std::fs;
use std::io;
use rand::distributions;
use rand::distributions::Distribution;
use rand::thread_rng;
use rand::seq::sample_slice_ref;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use weather::Weather;
use transport_mode::TransportMode;
use journey_type::JourneyType;
use neighbourhood::Neighbourhood;
use subculture::Subculture;
use scenario::Scenario;
use agent::Agent;
use statistics;
use hashmap_union::union_of;
use social_network;
use std::cmp;
use gaussian;

/// Run the simulation
/// * scenario: The scenario of the simulation
/// * number_of_people: The number of agents to generate
/// * social_connectivity: How connected the agent is to its social network
/// * subculture_connectivity: How connected the agent is to its subculture
/// * neighbourhood_connectivity: How connected the agent is to its neighbourhood
/// * number_of_neighbour_links: The minimum number of links each agent should have in the neighbourhood network
/// * days_in_habit_average: How many days should be used in the habit average
/// * weather_pattern: A HashMap from day number to Weather
/// * network: The social network
/// * Returns: Result, nothing if successful, io:Error if output could not be written
pub fn run(id: String,
           scenario_file: File,
           total_years: u32,
           number_of_people: u32,
           social_connectivity: f32,
           subculture_connectivity: f32,
           neighbourhood_connectivity: f32,
           number_of_neighbour_links: u32,
           days_in_habit_average: u32,
           distributions: Vec<(f64, f64, f64)>,
           weather_pattern: &Vec<Weather>,
           network: HashMap<u32, Vec<u32>>) -> Result<(), io::Error> 
{
    // Used for monitoring running time
    let t0 = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    // Load scenario
    let scenario = Scenario::from_file(scenario_file);

    // Create the agents
    let mut residents: Vec<Rc<RefCell<Agent>>> = set_up(
        &scenario, social_connectivity, subculture_connectivity,
        neighbourhood_connectivity, days_in_habit_average,
        number_of_neighbour_links, number_of_people, 
        distributions, network);

    // Report the setup running time
    let t1 = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    info!("[{}] Agents created in {}s", id, t1-t0);

    // Create the output directory if it does not already exist
    fs::create_dir_all("output")?;

    // Create the output file, and write the header to it
    let mut file = BufWriter::new(fs::File::create(format!("output/output_{}.csv", id))?);
    file.write(generate_csv_header(&scenario).as_bytes())?;

    // Get the weather at day 0
    let mut weather = weather_pattern[0];

    // Write the first set of statistics to the file
    file.write(generate_csv_output(0, &weather, &scenario, &residents).as_bytes())?;

    // For each day in the simulation
    for day in 1..total_years * 365 {
        // Intervene at the intervention day
        if day == scenario.intervention.day {
            intervene(&scenario, &residents)
        }

        // Only consider weekdays
        if weekday(day) {
            // Log the day to the terminal
            info!("[{}] Day: {}", id, day);

            // Get the new weather
            let new_weather = weather_pattern[day as usize];

            // For each resident update the habit
            for resident in residents.iter_mut() {
                resident.borrow_mut().update_habit();
            }

            // For each resident, choose a travel mode
            for resident in residents.iter_mut() {
                resident.borrow_mut().choose(&new_weather, weather != new_weather);
            }

            // Update the weather
            weather = new_weather;

            // Log the stats to the file
            file.write(generate_csv_output(day, &weather, &scenario, &residents).as_bytes())?;
        }
    }

    // Output the running time to the terminal
    let t2 = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    info!("[{}] Total elapsed time {}s", id, t2 - t0);
    info!("[{}] Total elapsed time excl. set up {}s", id, t2 - t1);

    Ok(())

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
/// * number_of_neighbour_links: The minimum number of links each agent should have in the neighbourhood network
/// * number_of_people: The number of agents to generate
/// * network: The social network
/// * Returns: The created agents
fn set_up(scenario: &Scenario,
          social_connectivity: f32,
          subculture_connectivity: f32,
          neighbourhood_connectivity: f32,
          days_in_habit_average: u32,
          number_of_neighbour_links: u32,
          number_of_people: u32,
          distributions: Vec<(f64, f64, f64)>,
          network: HashMap<u32, Vec<u32>>) -> Vec<Rc<RefCell<Agent>>> {
    // Create an empty vec to store agents
    let mut residents: Vec<Rc<RefCell<Agent>>> = Vec::new();
    // Create self.number_of_people unlinked agents
    for _ in 0..number_of_people {
        let agent: Agent = create_unlinked_agent(scenario, social_connectivity,
            subculture_connectivity, neighbourhood_connectivity, days_in_habit_average);

        residents.push(Rc::new(RefCell::new(agent)));
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
            &borrowed_agent.subculture, 
            borrowed_agent.social_connectivity, 
            borrowed_agent.commute_length, 
            &borrowed_agent.neighbourhood, 
            borrowed_agent.owns_car, 
            borrowed_agent.owns_bike);
        
        borrowed_agent.current_mode = new_mode;
        borrowed_agent.habit = hashmap!{new_mode => 1.0f32};
        borrowed_agent.norm = new_mode;
    }



    // Load pre-generated social networks
    link_agents_from_predefined_network(&mut residents, network, |agent, friends| agent.social_network.append(friends));
    residents.iter()
        .for_each(|a| debug!("Social network size for agent: {}", a.borrow().social_network.len()));

    // Group agents by neighbourhood
    let neighbourhood_residents: HashMap<String, Vec<Rc<RefCell<Agent>>>> = residents
        .iter()
        .map(|x| (x.borrow().neighbourhood.id.clone(), x.clone()))
        .into_group_map();

    // For each neighbourhood create a social network
    for (_, v) in neighbourhood_residents {
        link_agents_to_neighbours(&v, number_of_neighbour_links);
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
    let weather_sensitivity = 0.9f32;
    // TODO: Should consistency be used
    let consistency = 1.0f32;

    // Use a placeholder transport mode
    let current_mode: TransportMode = TransportMode::PublicTransport;
    let norm = current_mode;
    let last_mode = current_mode;

    // Create and return the agent
    Agent {
        subculture,
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
/// * subculture: The subculture of the agent
/// * subculture_connectivity: How connected the agent is to the subculture
/// * commute_length: The distance of the agent's commute
/// * neighbourhood: The neighbourhood of the agent
/// * Returns: The chosen transport mode
fn choose_initial_norm_and_habit(subculture: &Rc<Subculture>,
                                 subculture_connectivity: f32,
                                 commute_length: JourneyType,
                                 neighbourhood: &Rc<Neighbourhood>,
                                 owns_car: bool,
                                 owns_bike: bool
) -> TransportMode {
    let subculture_weight = subculture_connectivity;
    let mut initial_budget: HashMap<TransportMode, f32> = subculture
        .desirability
        .borrow()
        .iter()
        .map(|(&k, v)|(k, v * subculture_weight))
        .collect();

    // Find the key-value-pair in initial_budget with the highest value, and store the value
    let max: f32 = *initial_budget
        .iter()
        .max_by(|v1, v2| v1.1.partial_cmp(&v2.1).unwrap_or(cmp::Ordering::Equal))
        .unwrap()
        .1;

    // Make it so the the max mode has a budget of 1, therefore at least one mode is always possible
    initial_budget = initial_budget
        .into_iter()
        .map(|(k, v)| (k, v / max))
        .collect();

    // Take car / bike ownership into account
    let ownership_modifier = hashmap! {
        TransportMode::Car => if owns_car {1.0f32} else {0.0f32},
        TransportMode::Cycle => if owns_bike {1.0f32} else {0.0f32},
    };

    initial_budget = union_of(&initial_budget, &ownership_modifier, |v1, v2| v1 * v2);


    let commute_length_cost = commute_length.cost();

    // Take the supportiveness for each mode, away from 1, so a lower supportiveness = higher cost
    let neighbourhood_vals: HashMap<TransportMode, f32> = neighbourhood
        .supportiveness
        .borrow()
        .iter()
        .map(|(&k, v)| (k, 1.0f32 - v))
        .collect();

    let values_to_average: Vec<&HashMap<TransportMode, f32>> =
        vec![&neighbourhood_vals,
             &commute_length_cost];

    let initial_cost: HashMap<TransportMode, f32> = values_to_average
        .iter()
        .fold(HashMap::new(), |acc, x|
            union_of(&acc, x, |v1, v2| v1 + v2)
        )
        .into_iter()
        .map(|(k, v)| (k, v / (values_to_average.len() as f32)))
        .collect();

    // Filter out values where the budget is not greater than or equal to the cost
    // Calculate the difference between the budget and the cost
    // Get the maximum key-value pair (by value)
    // Set the current_mode equal to the key
    initial_budget
        .iter()
        .filter_map(|(&k, &v)| {
            let cost_val: f32 = *initial_cost.get(&k).unwrap_or(&9999.0f32);
            if v >= cost_val {
                Some((k, v - cost_val))
            } else {
                None
            }
        })
        .fold((TransportMode::Walk, -0.1),
              |(k0, v0): (TransportMode, f32), (k1, v1): (TransportMode, f32)| if v1 > v0 { (k1, v1) } else { (k0, v0) })
        .0
}

/// Link agents to a social network
/// <https://en.wikipedia.org/wiki/Barab%C3%A1si%E2%80%93Albert_model>
/// * agents: a slice of agents
/// * n: the minimum number of links
fn link_agents_to_neighbours(agents: &[Rc<RefCell<Agent>>], n: u32) {
    // Create network of ids
    let network = social_network::generate_social_network(n, agents.len() as u32);
    // Create the neighbourhood network from the network of ids
    link_agents_from_predefined_network(agents, network, |agent, friends| agent.neighbours.append(friends));

    agents.iter()
        .for_each(|a| debug!("Neighbourhood network size for agent: {}", a.borrow().neighbours.len()))
}

/// Generate the header for the csv file
/// * scenario: The scenario for this simulation
/// * Returns: The header for the csv file
fn generate_csv_header(scenario: &Scenario) -> String {
    let subculture_ids: Vec<String> = scenario
        .subcultures
        .iter()
        .map(|subculture| subculture.id.clone())
        .collect();

    let neighbourhood_ids: Vec<String> = scenario
        .neighbourhoods
        .iter()
        .map(|neighbourhood| neighbourhood.id.clone())
        .collect();

    format!(
        "Day,Rain,ActiveMode,ActiveNorm,ActiveModeCounterToInactiveNorm,InactiveModeCounterToActiveNorm,LocalCommute,CityCommute,DistantCommute,{},{}\n",
        subculture_ids.join(","),
        neighbourhood_ids.join(",")
    )
}

/// Generate CSV output that conforms to the header generated in generate_csv_header(...)
/// * day: The day number
/// * weather: The current weather
/// * scenario: The current scenario
/// * agents: The agents in the network
/// * Returns: The csv output for the day
fn generate_csv_output(day: u32, weather: &Weather, scenario: &Scenario, agents: &[Rc<RefCell<Agent>>]) -> String {
    let rain = if weather == &Weather::Good { 0 } else { 1 };

    let active_mode = statistics::count_active_mode(agents);
    let active_norm = statistics::count_active_norm(agents);
    let active_mode_counter_to_inactive_norm =
        statistics::count_active_mode_counter_to_inactive_norm(agents);

    let inactive_mode_counter_to_active_norm =
        statistics::count_inactive_mode_counter_to_active_norm(agents);

    let active_mode_by_commute_length = statistics::count_active_mode_by_commute_length(agents);
    let local_commute = active_mode_by_commute_length.get(&JourneyType::LocalCommute).unwrap();
    let city_commute = active_mode_by_commute_length.get(&JourneyType::CityCommute).unwrap();
    let distant_commute = active_mode_by_commute_length.get(&JourneyType::DistantCommute).unwrap();

    let active_mode_by_subculture =
        statistics::count_active_mode_by_subculture(agents);

    let active_mode_by_subculture_in_correct_order: Vec<String> = scenario
        .subcultures
        .iter()
        .map(|subculture| active_mode_by_subculture.get(subculture).unwrap_or(&0usize).to_string())
        .collect();

    let active_mode_by_neighbourhood =
        statistics::count_active_mode_by_neighbourhood(agents);

    let active_mode_by_neighbourhood_in_correct_order: Vec<String> = scenario
        .neighbourhoods
        .iter()
        .map(|neighbourhood| active_mode_by_neighbourhood.get(neighbourhood).unwrap_or(&0usize).to_string())
        .collect();

    format!(
        "{},{},{},{},{},{},{},{},{},{},{}\n",
        day,
        rain,
        active_mode,
        active_norm,
        active_mode_counter_to_inactive_norm,
        inactive_mode_counter_to_active_norm,
        local_commute,
        city_commute,
        distant_commute,
        active_mode_by_subculture_in_correct_order.join(","),
        active_mode_by_neighbourhood_in_correct_order.join(",")
    )
}

/// Link agents to a predefined social network
/// * agents: a slice of agents
/// * network: A map from agent id, to a vector of friend ids
/// * f: Should add friends to a network of agent.
fn link_agents_from_predefined_network(
    agents: &[Rc<RefCell<Agent>>], 
    network: HashMap<u32, Vec<u32>>, 
    f: fn(agent: &mut Agent, friends: &mut Vec<Rc<RefCell<Agent>>>))
{
    network
        .iter()
        .for_each(|(&k, v)| {
            // Load the agent's freinds
            let mut friends: Vec<Rc<RefCell<Agent>>> = v
                .iter()
                .map(|&id| agents[id as usize].clone())
                .collect();
            
            // Add the agent's friends to the network of the agent
            f(&mut agents[k as usize].borrow_mut(), &mut friends);
        });
}


/// Calculate whether a given day is a weekday
/// * day: The day number
/// * Returns: true iff day is a weekday
fn weekday(day: u32) -> bool {
    day % 7 < 5
}

/// This will run the intervention definined in scenario
/// * scenario: The scenario containing the interventions
/// * agents: The agents in the simulation
fn intervene(scenario: &Scenario, agents: &[Rc<RefCell<Agent>>]) {
    // This adds Intervention.neighbourhood_changes.increase_in_supportiveness 
    // to Neighbourhood.supportiveness
    scenario
        .intervention
        .neighbourhood_changes
        .iter()
        .for_each(
            |change| {
                let neighbourhood_to_change = scenario
                    .neighbourhoods
                    .iter()
                    .filter(|neighbourhood| neighbourhood.id == change.id)
                    .next()
                    .expect("A neighbourhood in your intervention was not found");

                let new_supportiveness = union_of(
                    &neighbourhood_to_change.supportiveness.borrow(),
                    &change.increase_in_supportiveness, 
                    |v1, v2| v1 + v2);

                neighbourhood_to_change
                    .supportiveness
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|(k, v)| *v = *new_supportiveness.get(&k).unwrap());
            }
        );

    // This adds Intervention.subculture_changes.increase_in_desirability 
    // to Subculture.desirability
    scenario
        .intervention
        .subculture_changes
        .iter()
        .for_each(
            |change| {
                let subculture_to_change = scenario
                    .subcultures
                    .iter()
                    .filter(|subculture| subculture.id == change.id)
                    .next()
                    .expect("A subculture in your intervention was not found");

                let new_desirability = union_of(
                    &subculture_to_change.desirability.borrow(),
                    &change.increase_in_desirability, 
                    |v1, v2| v1 + v2);    

                subculture_to_change
                    .desirability
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|(k, v)| *v = *new_desirability.get(&k).unwrap());
            }
        );
    
    if scenario.intervention.change_in_number_of_bikes > 0 {
        // Give people bikes
        
        // Filter agents without bikes
        let agents_without_bikes: Vec<&Rc<RefCell<Agent>>> = agents
            .iter()
            .filter(|agent| !agent.borrow().owns_bike)
            .collect();

        // Choose a random sample, the size of the increase, and give them bikes
        let mut rng = thread_rng();
        let sample = sample_slice_ref(
            &mut rng, 
            &agents_without_bikes, 
            scenario.intervention.change_in_number_of_bikes as usize);
        
        sample
            .iter()
            .for_each(|agent| agent.borrow_mut().owns_bike = true);
    } else if scenario.intervention.change_in_number_of_bikes < 0 {
        // Take away some bikes

        // Filter agents with bikes
        let agents_with_bikes: Vec<&Rc<RefCell<Agent>>> = agents
            .iter()
            .filter(|agent| agent.borrow().owns_bike)
            .collect();

        // Choose a random sample, the size of the decrease, and take away bikes
        let mut rng = thread_rng();
        let decrease_in_bikes = (scenario.intervention.change_in_number_of_bikes * -1) as usize;
        let sample = sample_slice_ref(
            &mut rng, 
            &agents_with_bikes, 
            decrease_in_bikes);
        
        sample
            .iter()
            .for_each(|agent| agent.borrow_mut().owns_bike = false);
    }

    if scenario.intervention.change_in_number_of_cars > 0 {
        // Give people cars

        // Filter agents without cars
        let agents_without_cars: Vec<&Rc<RefCell<Agent>>> = agents
            .iter()
            .filter(|agent| !agent.borrow().owns_car)
            .collect();

        // Choose a random sample, the size of the increase, and give them cars
        let mut rng = thread_rng();
        let sample = sample_slice_ref(
            &mut rng, 
            &agents_without_cars, 
            scenario.intervention.change_in_number_of_cars as usize);
        
        sample
            .iter()
            .for_each(|agent| agent.borrow_mut().owns_car = true);
    } else if scenario.intervention.change_in_number_of_cars < 0 {
        // Take away some cars

        // Filter agents with cars
        let agents_with_cars: Vec<&Rc<RefCell<Agent>>> = agents
            .iter()
            .filter(|agent| agent.borrow().owns_car)
            .collect();

        // Choose a random sample, the size of the decrease, and take away cars
        let mut rng = thread_rng();
        let decrease_in_cars = (scenario.intervention.change_in_number_of_cars * -1) as usize;
        let sample = sample_slice_ref(
            &mut rng, 
            &agents_with_cars, 
            decrease_in_cars);
        
        sample
            .iter()
            .for_each(|agent| agent.borrow_mut().owns_car = false);
    }
}
