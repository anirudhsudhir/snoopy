use serde::Deserialize;

#[derive(Deserialize)]
pub struct PeerConfig {
    pub interface: Interface,
    pub peer: Peer,
}

#[derive(Deserialize)]
pub struct Interface {
    pub name: String,
    pub virtual_address: String,
    pub endpoint: String,
}

#[derive(Deserialize)]
pub struct Peer {
    pub name: String,
    pub endpoint: String,
}

pub fn parse_config(config_path: &str) -> PeerConfig {
    let config = std::fs::read_to_string(config_path).expect("failed to read config from file");
    toml::from_str(&config).expect("failed to parse toml from config")
}
