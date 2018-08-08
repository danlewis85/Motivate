use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::rc::Rc;
use itertools::Itertools;
use std::cell::RefCell;
use transport_mode::TransportMode;
use agent::Agent;

// TODO: Is supportiveness the same as capacity
/// A Neighbourhood
#[derive(Clone, Serialize, Deserialize)]
pub struct Neighbourhood {
    /// The ID for the Neighbourhood, neighbourhoods are equal if they share the same id
    pub id: String,
    /// A score from 0-1 for each transport mode, on how supportive the environment is
    pub supportiveness: RefCell<HashMap<TransportMode, f32>>,
    /// The maximum capacity for a transport mode, at which there is no congestion
    pub capacity: RefCell<HashMap<TransportMode, u32>>,

    /// The calculated congestion modifier
    #[serde(skip, default = "default_congestion_modifier")]
    pub congestion_modifier: RefCell<HashMap<TransportMode, f32>>,

    /// The residents who live in this neighbourhood  
    /// Essentially this is a RefCell so that the Vec inside can be changed  
    /// This then contains reference counter (immutable) pointers to RefCell  
    /// That are mutable on the side
    #[serde(skip)]
    pub residents: RefCell<Vec<Rc<RefCell<Agent>>>>
}

/// This returns a default congestion modifier of 1.0 for every mode
fn default_congestion_modifier() -> RefCell<HashMap<TransportMode, f32>> {
    RefCell::new(hashmap! {
        TransportMode::Car => 1.0,
        TransportMode::Cycle => 1.0,
        TransportMode::PublicTransport => 1.0,
        TransportMode::Walk => 1.0
    })
}

impl Default for Neighbourhood {
    fn default() -> Self {
        Neighbourhood {
            id: "placeholder".to_string(),
            supportiveness: RefCell::new(HashMap::new()),
            capacity: RefCell::new(HashMap::new()),
            congestion_modifier: RefCell::new(HashMap::new()),
            residents: RefCell::new(Vec::new())
        }
    }
}

impl Neighbourhood {
    /// This updates the congestion modifier
    pub fn update_congestion_modifier(&self) {
        // Group agents by last mode
        // Count the agents
        // map ->
        //  if count <= capacity => 1.0
        //  else =>
        //    maximum_excess_demand = agents_in_neighbourhood.len() - capacity
        //    actual_excess_demand = count - capacity
        //    1.0 - (actual_excess_demand / maximum_excess_demand)
        let agents_in_neighbourhood = &self.residents.borrow();

        let new_congestion_modifier: HashMap<TransportMode, f32> = self
            .residents
            .borrow()
            .iter()
            .map(|agent| (agent.borrow().last_mode, Rc::clone(agent)))
            .into_group_map()
            .into_iter()
            .map(|(mode, grouped_agents)| (mode, grouped_agents.len()))
            .map(|(mode, count)| {
                let capacity_for_mode = *self.capacity.borrow().get(&mode).unwrap() as usize;
                if count <= capacity_for_mode {
                    (mode, 1.0)
                } else {
                    let maximum_excess_demand: usize = agents_in_neighbourhood.len() - capacity_for_mode;
                    let actual_excess_demand: usize = count - capacity_for_mode;
                    (mode, 1.0 - (actual_excess_demand as f32 / maximum_excess_demand as f32))
                }
            })
            .collect();

        self.congestion_modifier.replace(new_congestion_modifier);
    }
}

impl PartialEq for Neighbourhood {
    /// Tests equality of neighbourhood's only equal if they have the same id
    fn eq(&self, other: &Neighbourhood) -> bool {
        self.id == other.id
    }
}

impl Eq for Neighbourhood {}

impl Hash for Neighbourhood {
    /// Returns the has of the id
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// Congestion for road space should be able to be represented with a supply / demand curve
//
//       demand         supply                   
// price | \              |                                               
//   of  |  \             |                                               
//  road |   \            |                                               
// space |    \           |                                               
//       |     \          |                                               
//       |      \         |                                               
//       |       \        |                                               
//       |        \       |                                               
//       |         \      |                                               
//       |          \     |                                               
//       |           \    |                                               
//       |            \   |                                               
//       |             \  |                                               
//       |              \ |                                               
//       |               \|                                               
//  Pe ->|                | <- Free market road space                                             
//       |                |\                                              
//       |                | \                                             
//       |                |  \                                            
//       |                |   \                                           
//       |                |    \                                          
//       |                |     \                                         
//       |                |      \                                        
//       |                |       \                                       
//       |                |        \                                      
//       |                |         \                                     
//       |                |          \                                    
//       |                |           \                                   
//       |                |            \                                  
//       |                |             \                                 
//       |                |              \                                
//       |                |               \                               
//  P0 ->+----------------|-----------------------------------------------
//                        ^               ^              Quantity of road space
//                        |               |
//                        Qe              Q0
//
// As the price of road space is free, this leads to a demand for road space at Q0, but only a supply at Qe,
// therefore there is a shortage in supply of road space, which is congestion.
//
// Neighbourhood.capacity is the supply curve, unsure of what a realistic demand curve would look like, other than
// it being price inelastic
//
// If we create a mapping from price of road space, where at Q <= Qe, there is a value (name?) of 1.0, and then at Q0
// there is a value of 0.0. Should Q0 be the maximum number in the neighbourhood (so 7500)?
//
// Congestion for public transport should follow the same graph, substuting road space for public transport space?
