// TODO: validate configuration parameters strictly at runtime

use serde::Deserialize;

use std::net::IpAddr;

#[derive(Deserialize)]
pub struct Config {
    pub interface: Interface,
    pub peer: Peer,
}

#[derive(Deserialize)]
pub struct Interface {
    pub name: String,
    pub virtual_address: IpAddr,
    pub virtual_netmask: IpAddr,
    pub endpoint: String,
}

#[derive(Deserialize)]
pub struct Peer {
    pub name: String,
    pub endpoint: String,
}

pub fn parse_config(config_path: &str) -> Config {
    let config = std::fs::read_to_string(config_path).expect("failed to read config from file");
    toml::from_str(&config).expect("failed to parse toml from config")
    // validate_conf(&mut conf);
    // conf
}

// fn validate_conf(config: &mut Config) -> Result<(), String> {
//     Ok(())
// }
