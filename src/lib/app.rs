use nostr_ndb::NdbDatabase;

use crate::config;

pub struct AppState {
    pub config: config::Config,
    pub db: NdbDatabase,
}
