use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Limitations {
    pub max_message_length: u64,
    pub max_subscriptions: u64,
    pub max_filters: u64,
    pub max_limit: u64,
    pub max_subid_length: u64,
    pub max_event_tags: u64,
    pub max_content_length: u64,
    pub min_pow_difficulty: u64,
    pub auth_required: bool,
    pub payment_required: bool,
    pub restricted_writes: bool,
    pub created_at_lower_limit: u64,
    pub created_at_upper_limit: u64,
}

impl Default for Limitations {
    fn default() -> Self {
        // These are sensible defaults defined in:
        // https://github.com/nostr-protocol/nips/blob/master/11.md#server-limitations
        Self {
            max_message_length: 16384,
            max_subscriptions: 20,
            max_filters: 100,
            max_limit: 5000,
            max_subid_length: 100,
            max_event_tags: 100,
            max_content_length: 8196,
            min_pow_difficulty: 30,
            auth_required: false,
            payment_required: false,
            restricted_writes: false,
            created_at_lower_limit: 31536000,
            created_at_upper_limit: 3,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub banner: String,
    pub icon: String,
    pub pubkey: String,
    pub contact: String,
    pub supported_nips: Vec<(u16, String)>,
    pub software: String,
    pub version: String,
    pub relay_port: u16,
    pub relay_bind_address: String,
    #[serde(default)]
    pub limits: Limitations,
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
        assert!(!config.name.is_empty());
        assert!(!config.description.is_empty());
        assert!(!config.banner.is_empty());
        assert!(!config.icon.is_empty());
        assert!(!config.pubkey.is_empty());
        assert!(!config.contact.is_empty());
        assert!(!config.software.is_empty());
        assert!(!config.version.is_empty());
        assert!(config.relay_port > 0);
    }
}
