use rand;
use std::collections::HashMap;

/// The weather for a given day
#[derive(Eq, Hash, PartialEq, Copy, Clone)]
pub enum Weather {
    Good,
    Bad
}

impl Weather {
    /// This generates a weather pattern using a Markov Chain
    /// * transition_matrix: This defines if in state x, the chance of moving to state y is z, where
    /// x & y are weather conditions that may be equal
    /// * chance_of_rain: The chance of rain on day 0
    /// * days: The number of days to generate
    /// * Returns: A Vec<Weather> where the index is the day
    pub fn make_pattern(
        transition_matrix: HashMap<Weather, HashMap<Weather, f64>>,
        chance_of_rain: f64,
        days: usize) -> Vec<Self> 
    {
        // Create an empty weather pattern
        let mut pattern = Vec::with_capacity(days);
        // On day 0, calculate if their is rain
        if rand::random::<f64>() > chance_of_rain {
            pattern.push(Weather::Good);
        } else {
            pattern.push(Weather::Bad);
        }

        // For each day
        for i in 1..days {
            // Using the weather from the previous day,
            // get the probability of good weather, 
            // calculate a random float, if this is less
            // than that probability then the weather for day i
            // is good
            if rand::random::<f64>() < 
                *transition_matrix.get(&pattern[i - 1]).unwrap()
                .get(&Weather::Good).unwrap() {
                    pattern.push(Weather::Good);
                } else {
                    pattern.push(Weather::Bad);
                }
        }
        pattern
    }
}