use async_trait::async_trait;

use super::runner::{VRunner, VRunnerError};
use crate::cqrs::{Aggregate, AggregateContext, CqrsFramework, EventStore, Query};
use crate::{std::wallet::aggregate, utils};
use futures::executor;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppPermission {
    name: String,
    description: String,
    app: String, // app
    cmds: Vec<String>,
    events: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub permission: Vec<AppPermission>,
}
enum VIPCError {
    Unknown,
}

pub trait VIPC<A> {
    fn main_service() -> A;
    fn exec(cmd: String, value: serde_json::Value) -> Result<(), VIPCError>;
}

pub trait VAggregate:
    Aggregate<Services = <Self as VAggregate>::Services, Command = <Self as VAggregate>::Command>
{
    type Services: Sync + Send;
    type Command: DeserializeOwned;
}

#[derive(Debug)]
pub enum EventFactoyError {}

pub trait VEventStoreFactory {
    fn build<'a, A: Aggregate>(&self, id: &'a str) -> Result<impl EventStore<A>, EventFactoyError>;
}

#[derive(Default)]
pub struct VAppBuilder<A: Aggregate, AC: AggregateContext<A>> {
    queries: Option<Vec<Box<dyn Query<A::Event>>>>,
    store: Option<Box<dyn EventStore<A, AC = AC>>>,
    services: Option<Box<A::Services>>,
    app_info: Option<AppInfo>,
}

impl<A: Aggregate, AC: AggregateContext<A>> VAppBuilder<A, AC> {
    fn with_app_info(self, app_info: AppInfo) -> Self {
        Self {
            app_info: Some(app_info),
            ..self
        }
    }

    fn with_services(self, services: Box<A::Services>) -> Self {
        Self {
            services: Some(services),
            ..self
        }
    }

    fn with_queries(self, queries: Vec<Box<dyn Query<A::Event>>>) -> Self {
        Self {
            queries: Some(queries),
            ..self
        }
    }

    fn with_store(self, store: Box<dyn EventStore<A, AC = AC>>) -> Self {
        Self {
            store: Some(store),
            ..self
        }
    }
}

pub struct VApp<A: Aggregate, E: EventStore<A>> {
    app_info: AppInfo,
    cqrs: CqrsFramework<A, E>,
}

impl<A: Aggregate, E: EventStore<A>> VApp<A, E> {
    pub fn new(
        app_info: AppInfo,
        event_store: E,
        queries: Vec<Box<dyn Query<A::Event>>>,
        services: A::Services,
    ) -> Self {
        Self {
            app_info,
            cqrs: CqrsFramework::new(event_store, queries, services),
        }
    }
}

#[async_trait]
impl<A: Aggregate, E: EventStore<A>> VRunner for VApp<A, E> {
    async fn exec<'a>(
        &self,
        aggregate_id: &'a str,
        command: serde_json::Value,
        metadata: utils::HashMap<String, String>,
    ) -> Result<(), VRunnerError> {
        let command: A::Command =
            serde_json::from_value(command).map_err(|_| VRunnerError::Unknown)?;
        self.cqrs
            .execute(aggregate_id, command)
            .await
            .map_err(|_| VRunnerError::Unknown)
    }
}
