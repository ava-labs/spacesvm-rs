#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub fn get_network_runner_grpc_endpoint() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_GRPC_ENDPOINT") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

pub fn get_network_runner_avalanchego_path() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_AVALANCHEGO_PATH") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

pub fn get_network_runner_whitelisted_subnets() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_WHITELISTED_SUBNETS") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

pub fn get_network_runner_plugin_dir_path() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_PLUGIN_DIR_PATH") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

pub fn get_custom_vms() -> (HashMap<String, String>, bool) {
    match std::env::var("NETWORK_RUNNER_CUSTOM_VM") {
        Ok(s) => (hash_from_str(&s), true),
        _ => (HashMap::new(), false),
    }
}

/// hash_from_str takes a comma delimited key value pair and converts it to a hashmap.
/// example: key=value,key=value.
fn hash_from_str(input: &str) -> HashMap<String, String> {
    input
        .split(',')
        .map(|s| s.split_at(s.find("=").expect("invalid format expect key=value")))
        .map(|(key, val)| (key.to_string(), val[1..].to_string()))
        .collect()
}
