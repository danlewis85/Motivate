use std::collections::HashMap;
use transport_mode::TransportMode;
use itertools::Itertools;

/// This is a debug function that prints to the debug log level a transport mode hashmap
/// * prefix: The prefix before the output
/// * hashmap: The hashmap to print
pub fn print_transport_mode_hashmap(prefix: &str, hashmap: &HashMap<TransportMode, f32>) {
    let hashmap_str = hashmap
        .iter()
        .map(|(mode, value)| match *mode {
            TransportMode::Cycle => format!("Cycle: {}", value),
            TransportMode::Car => format!("Car: {}", value),
            TransportMode::Walk => format!("Walk: {}", value),
            TransportMode::PublicTransport => format!("PT: {}", value)
        })
        .intersperse(",".to_string())
        .collect::<Vec<_>>()
        .concat();
    
    debug!("{}: {}", prefix, hashmap_str);
}