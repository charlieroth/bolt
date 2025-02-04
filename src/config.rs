use serde::{Deserialize, Serialize};
use std::fs::File;
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub owner_npub: String,
    pub relay_port: u16,
    pub relay_bind_address: String,
}

impl Config {
    pub fn new(path: &str) -> Result<Self, serde_yml::Error> {
        let config_file = File::open(path).unwrap();
        let config = serde_yml::from_reader(config_file).unwrap();
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_config_from_given_path() {
        let config = Config::new("config.yml").unwrap();
        assert!(!config.owner_npub.is_empty());
        assert!(config.relay_port > 0);
        assert!(!config.relay_bind_address.is_empty());
    }
}
