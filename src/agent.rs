use std::collections::HashMap;
use itertools::Itertools;
use std::rc::Rc;
use std::cell::RefCell;
use std::cmp;
use weather::Weather;
use transport_mode::TransportMode;
use journey_type::JourneyType;
use neighbourhood::Neighbourhood;
use subculture::Subculture;
use hashmap_union::union_of;

/// The agent in the model
#[derive(PartialEq)]
pub struct Agent {
    /// The demographic agent of the subculture
    pub subculture: Rc<Subculture>,

    /// The neighbourhood the agent lives in
    pub neighbourhood: Rc<Neighbourhood>,

    /// The distance of the agent's commute (categorical).  
    /// This may become deprecated, once commute_length_continuous
    /// has a corresponding cost function
    pub commute_length: JourneyType,

    /// The distance of the agent's commute.  
    /// This is currently unused, but should correspond to the
    /// categorical commute_length
    pub commute_length_continuous: f64,

    /// How sensitive the agent is to the weather
    pub weather_sensitivity: f32,
    
    /// How consistent the agent is (used as a weighting for habit)
    pub consistency: f32,

    /// How connected the agent is to its social network
    pub social_connectivity: f32,

    /// How connected the agent is to its subculture
    pub subculture_connectivity: f32,

    /// How connected the agent is to its neighbourhood
    pub neighbourhood_connectivity: f32,

    /// The weight used for the average
    pub average_weight: f32,

    /// The habit of the agent, mapping the TransportMode, to a
    /// recency-weighted average where 1 was used, if the TransportMode
    /// was used on a given day, 0 if it was not.
    pub habit: HashMap<TransportMode, f32>,

    /// How the agent is currently travelling to work
    pub current_mode: TransportMode,

    /// How the agent travelled to work on the previous day
    pub last_mode: TransportMode,

    /// The maximum of the joint effects of social network, subculture
    /// and neighbours, appropriately weighted
    pub norm: TransportMode,

    /// Whether the agent owns a bike
    pub owns_bike: bool,

    /// Whether the agent owns a car
    pub owns_car: bool,

    /// The friends of the agent
    pub social_network: Vec<Rc<RefCell<Agent>>>,

    /// Neighbours of the agent
    pub neighbours: Vec<Rc<RefCell<Agent>>>
}

impl Agent {
    /// Calculate the mode budget for the agent  
    /// Will also update the norm
    /// * Returns: the mode budget, a map from TransportMode to values
    fn calculate_mode_budget(&mut self) -> HashMap<TransportMode, f32> {
        // This is the percentage of people in the social network who take a given TransportMode,
        // Weighted by social connectivity
        let social_vals =
            count_in_subgroup(&self.social_network, self.social_connectivity);

        // This is the percentage of neighbours who take a given TransportMode,
        // Weighted by neighbourhood connectivity
        let neighbour_vals =
            count_in_subgroup(&self.neighbours, self.neighbourhood_connectivity);

        // This is the subculture desirability weighted by subculture connectivity
        let subculture_vals: HashMap<TransportMode, f32> = self.subculture
            .desirability
            .borrow()
            .iter()
            .map(
                |(&k, &v)| (k, v * self.subculture_connectivity)
            ).collect();

        let values_to_add: Vec<&HashMap<TransportMode, f32>> =
            vec![&social_vals, &neighbour_vals, &subculture_vals];

        // This sets the norm as the maximum of social_vals + neighbour_vals + subculture_vals
        self.norm = *values_to_add
                .iter()
                .fold(HashMap::new(), |acc, x|
                    union_of(&acc, x, |v1, v2| v1 + v2)
                )
                .iter()
                .max_by(|v1, v2| v1.1.partial_cmp(&v2.1).unwrap_or(cmp::Ordering::Equal))
                .unwrap()
                .0;

        // Weight habit by consistenct
        let habit_vals: HashMap<TransportMode, f32> = self.habit.iter().map(
            |(&k, &v)| (k, v * self.consistency)
        ).collect();

        // Averages social_vals, neighbour_vals, subculture_vals, habit
        let values_to_average: Vec<&HashMap<TransportMode, f32>> =
            vec![&social_vals, &neighbour_vals, &subculture_vals, &habit_vals];

        // This is the average of the values above
        let mut average: HashMap<TransportMode, f32> = values_to_average
            .iter()
            .fold(HashMap::new(), |acc, x|
                union_of(&acc, x, |v1, v2| v1 + v2)
            )
            .into_iter()
            .map(|(k, v)| (k, v / (values_to_average.len() as f32)))
            .collect();

        // Take car / bike ownership into account
        let ownership_modifier = hashmap! {
            TransportMode::Car => if self.owns_car {1.0f32} else {0.0f32},
            TransportMode::Cycle => if self.owns_bike {1.0f32} else {0.0f32},
        };

        average = union_of(&average, &ownership_modifier, |v1, v2| v1 * v2);

        // Find the key-value-pair in average with the highest value, and store the value
        let max: f32 = *average
            .iter()
            .max_by(|v1, v2| v1.1.partial_cmp(&v2.1).unwrap_or(cmp::Ordering::Equal))
            .unwrap()
            .1;

        // Make it so the the max mode has a budget of 1, therefore at least one mode is always possible
        // Return the result
        average
            .into_iter()
            .map(|(k, v)| (k, v / max))
            .collect()
    }

    /// Calculate the cost of travel
    /// * weather: The current weather
    /// * change_in_weather: true if there has been a change in the weather, false otherwise
    /// * Returns: A Map from transport mode to its cost
    fn calculate_cost(&self, weather: &Weather, change_in_weather: bool) -> HashMap<TransportMode, f32> {
        // Take the supportiveness for each mode, away from 1, so a lower supportiveness = higher cost
        let neighbourhood_vals: HashMap<TransportMode, f32> = self.neighbourhood
            .supportiveness
            .borrow()
            .iter()
            .map(|(&k, v)| (k, 1.0f32 - v))
            .collect();

        // Get the cost for the commute
        let commute_cost = self.commute_length.cost();

        // Average commute_cost and neighbourhood_vals
        let values_to_average =
            vec!(&commute_cost, &neighbourhood_vals);

        let mut average: HashMap<TransportMode, f32> = values_to_average
            .iter()
            .fold(HashMap::new(), |acc, x| union_of(&acc, x, |v1, v2| v1 + v2))
            .iter()
            .map(|(&k, v)| (k, v / (values_to_average.len()) as f32))
            .collect();

        // If the weather is bad this has an impact on the cost
        if weather == &Weather::Bad {
            // If the weather was bad the day previous, and you took an active mode, this strengthens
            // your resolve to do so again, reducing the cost of an active mode, otherwise your resolve
            // is weakened.
            let resolve = if !change_in_weather &&
                (self.last_mode == TransportMode::Walk || self.last_mode == TransportMode::Cycle) {
                -0.1f32
            } else if !change_in_weather {
                0.1f32
            } else {
                0.0f32
            };

            // Calculate the weather modifier for Cycling and Walking
            let weather_modifier = hashmap!{
                TransportMode::Cycle => 1.0f32 + self.weather_sensitivity + resolve,
                TransportMode::Walk => 1.0f32 + self.weather_sensitivity + resolve
            };

            // Multiply the average by the weather modifier
            let values_to_multiply = vec![average.clone(), weather_modifier];

            average = values_to_multiply
                .iter()
                .fold(HashMap::new(), |acc, x| union_of(&acc, x, |v1, v2| v1 * v2));
        }
        

        // If the cost is > 1, make the cost 1?
        average
            .into_iter()
            .map(|(k, v)| (k, if v <= 1.0 {v} else {1.0}))
            .collect()
    }

    /// Updates the habit, should be called at the start of each day,
    /// Also updates last_mode
    pub fn update_habit(&mut self) {
        self.last_mode = self.current_mode;

        // the average_weight (technically multiplied by 1) should be added to
        // (1 - average_weight) * habit, to get the new habit
        let last_mode_map: HashMap<TransportMode, f32> = hashmap!{
            self.last_mode => self.average_weight
        };
        // Multiply the existing habit, by (1 - average_weight)
        let old_habit: HashMap<TransportMode, f32> = self.habit
            .iter()
            .map(|(&k, v)| (k, v * (1.0f32 - self.average_weight)))
            .collect();

        // Add the old_habit and last_mode_map to create the new habit
        let values_to_add = vec![old_habit, last_mode_map];

        self.habit = values_to_add
            .iter()
            .fold(HashMap::new(), |acc, x| union_of(&acc, x, |v1, v2| v1 + v2));
    }

    /// Choose a mode of travel
    /// * weather: The current weather
    /// * change_in_weather: true if there has been a change in the weather, false otherwise
    pub fn choose(&mut self, weather: &Weather, change_in_weather: bool) {
        // Get the budget and cost
        let budget = self.calculate_mode_budget();
        let cost = self.calculate_cost(weather, change_in_weather);
        
        if (*budget.get(&TransportMode::Car).unwrap() != 0.0f32 && !self.owns_car) ||
            (*budget.get(&TransportMode::Cycle).unwrap() != 0.0f32 && !self.owns_bike) {
                println!("Bad Unexpected things have occured");
        };

        // Filter out values where the budget is not greater than or equal to the cost
        // Calculate the difference between the budget and the cost
        // Get the maximum key-value pair (by value)
        // Set the current_mode equal to the key
        budget
            .iter()
            .filter_map(|(&k, &v)| {
                let cost_val: f32 = *cost.get(&k).unwrap_or(&99999999.0f32);
                if v >= cost_val {
                    Some((k, v - cost_val))
                } else {
                    None
                }
            })
            .max_by(|v1, v2| v1.1.partial_cmp(&v2.1).unwrap_or(cmp::Ordering::Equal))
            .unwrap()
            .0;
    }
}

/// Counts in a subgroup of Agents, the percentage of people taking each travel mode
/// * agents: the subgroup of agents
/// * weight: the weight to apply to the percentage
fn count_in_subgroup(agents: &[Rc<RefCell<Agent>>], weight: f32) -> HashMap<TransportMode, f32> {
    // Group them by travel mode, then calculate the percentage (multiplied by the weight)
    let agents_size = agents.len() as f32;
    agents
        .iter()
        .map(|x| (x.borrow().last_mode, 1))
        .into_group_map()
        .iter()
        .map(|(&k, v)| (k, (v.len() as f32) * weight / agents_size))
        .collect()
}
