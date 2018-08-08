use transport_mode::TransportMode;
use std::collections::HashMap;

/// This defined an intervention that can take place
#[derive(Clone, Serialize, Deserialize)]
pub struct Intervention {
    /// The day number of the intervention
    pub day: u32,
    
    /// Changes in the neighbourhood
    pub neighbourhood_changes: Vec<NeighbourhoodChange>,

    /// Changes in the subculture
    pub subculture_changes: Vec<SubcultureChange>,

    /// Change in the number of bikes
    pub change_in_number_of_bikes: i32,

    /// Change in the number of cars
    pub change_in_number_of_cars: i32,
}

/// This defines changes in the neighbourhood that may form part of an intervention
#[derive(Clone, Serialize, Deserialize)]
pub struct NeighbourhoodChange {
    /// Neighbourhood ID
    pub id: String,

    /// Be very careful that this does not make the supportiveness > 1 or < 0  
    /// This represents an increase in supportiveness of v for TransportMode k, 
    /// where (k, v) are elements of the HashMap
    #[serde(default)]
    pub increase_in_supportiveness: HashMap<TransportMode, f32>,

    /// Be very careful that this does not make capacity > the max size of u32, or < 0  
    /// This represents an increase in capacity of v for TransportMode k,
    /// where (k, v) are elements of the HashMap
    #[serde(default)]
    pub increase_in_capacity: HashMap<TransportMode, i64>
}

/// This defines changes to a subculture that may form part of an intervention
#[derive(Clone, Serialize, Deserialize)]
pub struct SubcultureChange {
    /// Subculture ID
    pub id: String,

    /// Be very careful that this does not make the desirability > 1 or < 0
    /// this represents an increase in desirability of v for TransportMode k, 
    /// where (k, v) are elements of the HashMap
    pub increase_in_desirability: HashMap<TransportMode, f32>
}