use std::rc::Rc;
use std::fs::File;
use std::io::prelude::*;
use subculture::Subculture;
use serde_yaml;
use neighbourhood::Neighbourhood;
use intervention::Intervention;

/// A scenario for a simulation run
#[derive(Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// The scenario ID
    pub id: String,
    /// The subcultures in the scenario
    pub subcultures: Vec<Rc<Subculture>>,
    /// The neighbourhoods in the scenario
    pub neighbourhoods: Vec<Rc<Neighbourhood>>,

    pub number_of_bikes: u32,

    pub number_of_cars: u32,

    /// The intervention
    pub intervention: Intervention
}

impl Scenario {
    pub fn from_file(mut file: File) -> Self {
        info!("Loading scenario from file");
        let mut file_contents = String::new();

        file.read_to_string(&mut file_contents)
            .expect("There was an error reading the file");

        serde_yaml::from_slice(file_contents.as_bytes())
            .expect("There was an error parsing the file")
    }
}
