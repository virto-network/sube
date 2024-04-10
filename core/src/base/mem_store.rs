use crate::utils::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use super::{Aggregate, AggregateContext, AggregateError, DomainEvent, EventEnvelope, EventStore};

#[derive(Debug, Clone)]
pub struct MemStore<A: Aggregate + Send + Sync> {
    events: Arc<LockedEventEnvelopeMap<A::Event>>,
}

impl<A: Aggregate> Default for MemStore<A> {
    fn default() -> Self {
        let events = Arc::default();
        Self { events }
    }
}

type LockedEventEnvelopeMap<Event> = RwLock<HashMap<String, Vec<EventEnvelope<Event>>>>;

impl<A: Aggregate> MemStore<A> {
    #[deprecated(since = "0.4.9", note = "clone the MemStore instead")]
    pub fn get_events(&self) -> Arc<LockedEventEnvelopeMap<A::Event>> {
        Arc::clone(&self.events)
    }

    fn load_commited_events(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<EventEnvelope<A::Event>>, AggregateError<A::Error>> {
        let event_map = self.events.read().unwrap();
        let mut committed_events: Vec<EventEnvelope<A::Event>> = Vec::new();
        for event in event_map.get(aggregate_id).into_iter().flatten() {
            committed_events.push(event.clone());
        }
        Ok(committed_events)
    }

    fn aggregate_id(&self, events: &[EventEnvelope<A::Event>]) -> String {
        let &first_event = events.iter().peekable().peek().unwrap();
        first_event.aggregate_id.to_string()
    }
}

#[async_trait]
impl<A: Aggregate> EventStore<A> for MemStore<A> {
    type AC = MemStoreAggregateContext<A>;

    async fn load_events(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<EventEnvelope<A::Event>>, AggregateError<A::Error>> {
        let events = self.load_commited_events(aggregate_id)?;
        println!(
            "loading: {} events for aggregate ID '{}'",
            &events.len(),
            &aggregate_id
        );
        Ok(events)
    }

    async fn load_aggregate(
        &self,
        aggregate_id: &str,
    ) -> Result<MemStoreAggregateContext<A>, AggregateError<A::Error>> {
        let committed_events = self.load_events(aggregate_id).await?;
        let mut aggregate = A::default();
        let mut current_sequence = 0;
        for envelope in committed_events {
            current_sequence = envelope.sequence;
            let event = envelope.payload;
            aggregate.apply(event);
        }
        Ok(MemStoreAggregateContext {
            aggregate_id: aggregate_id.to_string(),
            aggregate,
            current_sequence,
        })
    }

    async fn commit(
        &self,
        events: Vec<A::Event>,
        context: MemStoreAggregateContext<A>,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<EventEnvelope<A::Event>>, AggregateError<A::Error>> {
        let aggregate_id = context.aggregate_id;
        let current_sequence = context.current_sequence;
        let wrapped_events = self.wrap_events(&aggregate_id, current_sequence, events, metadata);
        let new_events_qty = wrapped_events.len();
        if new_events_qty == 0 {
            return Ok(Vec::default());
        }
        let aggregate_id = self.aggregate_id(&wrapped_events);
        let mut new_events = self.load_commited_events(&aggregate_id).unwrap();
        for event in &wrapped_events {
            new_events.push(event.clone());
        }
        println!(
            "storing: {} new events for aggregate ID '{}'",
            new_events_qty, &aggregate_id
        );

        self.events
            .write()
            .unwrap()
            .insert(aggregate_id, new_events);
        Ok(wrapped_events)
    }
}

impl<A: Aggregate> MemStore<A> {
    fn wrap_events(
        &self,
        aggregate_id: &str,
        current_sequence: usize,
        resultant_events: Vec<A::Event>,
        base_metadata: HashMap<String, String>,
    ) -> Vec<EventEnvelope<A::Event>> {
        let mut sequence = current_sequence;
        resultant_events
            .into_iter()
            .map(|payload| {
                sequence += 1;
                EventEnvelope {
                    aggregate_id: aggregate_id.to_string(),
                    sequence,
                    payload,
                    metadata: base_metadata.clone(),
                }
            })
            .collect()
    }
}
pub struct MemStoreAggregateContext<A>
where
    A: Aggregate,
{
    pub aggregate_id: String,
    pub aggregate: A,
    pub current_sequence: usize,
}

impl<A> AggregateContext<A> for MemStoreAggregateContext<A>
where
    A: Aggregate,
{
    fn aggregate(&self) -> &A {
        &self.aggregate
    }
}
