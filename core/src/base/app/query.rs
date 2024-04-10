use super::types::{DomainEvent, EventEnvelope, StateMachine};
use crate::utils::prelude::*;

pub trait Query<Event>: Send + Sync
where
    Event: DomainEvent,
{
    async fn dispatch(&self, aggregate_id: &str, events: &[EventEnvelope<Event>]);
}
