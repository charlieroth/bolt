use nostr::{event::Event, filter::Filter};
use nostr_ndb::{
    database::{DatabaseError, Events, NostrEventsDatabase, SaveEventStatus},
    NdbDatabase,
};
use std::future::Future;

pub trait Relay {
    fn store_event(
        &self,
        event: Event,
    ) -> impl Future<Output = Result<SaveEventStatus, DatabaseError>>;
    fn delete_event(&self, filter: Filter) -> impl Future<Output = Result<(), DatabaseError>>;
    fn query_events(&self, filter: Filter) -> impl Future<Output = Result<Events, DatabaseError>>;
}

pub struct MemoryRelay {
    store: NdbDatabase,
}

impl MemoryRelay {
    pub fn new() -> Result<Self, DatabaseError> {
        let store = NdbDatabase::open("./relay.db")?;
        Ok(Self { store })
    }
}

impl Relay for MemoryRelay {
    async fn store_event(&self, event: Event) -> Result<SaveEventStatus, DatabaseError> {
        let status = self.store.save_event(&event).await?;
        Ok(status)
    }

    async fn delete_event(&self, filter: Filter) -> Result<(), DatabaseError> {
        self.store.delete(filter).await
    }

    async fn query_events(&self, filter: Filter) -> Result<Events, DatabaseError> {
        let events = self.store.query(filter).await?;
        Ok(events)
    }
}
