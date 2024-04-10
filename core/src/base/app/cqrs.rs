use crate::utils::HashMap;

use super::aggregate::Aggregate;
use super::error::AggregateError;
use super::query::Query;
use super::store::AggregateContext;
use super::store::EventStore;

pub struct CqrsFramework<A, ES>
where
    A: Aggregate,
    ES: EventStore<A>,
{
    store: ES,
    queries: Vec<Box<dyn Query<A::Event>>>,
    service: A::Services,
}

impl<A, ES> CqrsFramework<A, ES>
where
    A: Aggregate,
    ES: EventStore<A>,
{
    pub fn new(store: ES, queries: Vec<Box<dyn Query<A::Event>>>, service: A::Services) -> Self
    where
        A: Aggregate,
        ES: EventStore<A>,
    {
        Self {
            store,
            queries,
            service,
        }
    }

    pub fn append_query(&mut self, query: Box<dyn Query<A::Event>>)
    where
        A: Aggregate,
        ES: EventStore<A>,
    {
        self.queries.push(query);
    }

    pub async fn execute(
        &self,
        aggregate_id: &str,
        command: A::Command,
    ) -> Result<(), AggregateError<A::Error>> {
        self.execute_with_metadata(aggregate_id, command, HashMap::new())
            .await
    }

    pub async fn execute_with_metadata(
        &self,
        aggregate_id: &str,
        command: A::Command,
        metadata: HashMap<String, String>,
    ) -> Result<(), AggregateError<A::Error>> {
        let aggregate_context = self.store.load_aggregate(aggregate_id).await?;
        let aggregate = aggregate_context.aggregate();
        
        let resultant_events = aggregate
            .handle(command, &self.service)
            .await
            .map_err(AggregateError::UserError)?;

        let committed_events = self
            .store
            .commit(resultant_events, aggregate_context, metadata)
            .await?;

        if committed_events.is_empty() {
            return Ok(());
        }
        for processor in &self.queries {
            let dispatch_events = committed_events.as_slice();
            processor.dispatch(aggregate_id, dispatch_events).await;
        }
        Ok(())
    }
}
