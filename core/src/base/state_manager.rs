use crate::utils::prelude::*;
use crate::{CommittedEventEnvelope, SerializedCommandEnvelope, SerializedEventEnvelope};
use async_trait::async_trait;

pub enum StateManagerError {
    Unknown
}

#[async_trait]
pub trait StateManager: Send + Sync {
    async fn load_events(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<SerializedEventEnvelope>, StateManagerError>;

    async fn load_state(
        &self,
        app_id: &str,
        aggregate_id: &str,
    ) -> Result<Option<Value>, StateManagerError>;

    async fn save_state(
        &self,
        app_id: &str,
        aggregate_id: &str,
        state: Value,
    ) -> Result<(), StateManagerError>;

    async fn commit(
        &self,
        app_id: &str,
        events: Vec<SerializedEventEnvelope>,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<CommittedEventEnvelope>, StateManagerError>;
}
