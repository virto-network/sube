use crate::utils::prelude::*;
pub trait DomainEvent: Serialize + DeserializeOwned + Clone + Debug + Send {
    fn event_type(&self) -> String;

    fn event_version(&self) -> String;
}

pub trait DomainCommand {
    fn command_name(&self) -> String;
    fn command_payload(&self) -> Value;
}

pub trait ConstructableService: Sync + Send {
    type Args: DeserializeOwned + Clone;
    type Service;

    fn new(args: Self::Args) -> Self::Service;
}

#[async_trait]
pub trait StateMachine: Default + Serialize + DeserializeOwned + Send {
    type Command: DeserializeOwned + Send + Debug + Serialize;

    type Event: DomainEvent + Send + PartialEq;

    type Error: core::error::Error;

    type Services: Send + Sync;

    async fn handle(
        &self,
        command: Self::Command,
        service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    fn apply(&mut self, event: Self::Event);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializedCommandEnvelope {
    pub app_id: String, // app-id
    pub aggregate_id: String,
    pub metadata: HashMap<String, String>, // { who, req_id }
    pub cmd_name: String,
    pub cmd_payload: Value,
}

#[derive(Debug)]
pub struct EventEnvelope<E>
where
    E: DomainEvent,
{
    /// The id of the aggregate instance.
    pub aggregate_id: String,
    /// The sequence number for an aggregate instance.
    pub sequence: usize,
    /// The event payload with all business information.
    pub payload: E,
    /// Additional metadata for use in auditing, logging or debugging purposes.
    pub metadata: HashMap<String, String>,
}

impl<E: DomainEvent> Clone for EventEnvelope<E> {
    fn clone(&self) -> Self {
        Self {
            aggregate_id: self.aggregate_id.clone(),
            sequence: self.sequence,
            payload: self.payload.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedEventEnvelope {
    pub app_id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub event_version: String,
    pub sequence: usize, // after commit we know the sequence for that event
    pub payload: Value,
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializedEventEnvelope {
    pub app_id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub event_version: String,
    pub payload: Value,
    pub metadata: Value,
}

pub fn to_serialized_event_envelope<E: DomainEvent>(
    app_id: impl Into<String>,
    aggregate_id: impl Into<String>,
    event: &E,
    metadata: &HashMap<String, String>,
) -> SerializedEventEnvelope {
    SerializedEventEnvelope {
        app_id: app_id.into(),
        aggregate_id: aggregate_id.into(),
        metadata: serde_json::to_value(metadata).expect("invalid metadata"),
        payload: serde_json::to_value(event).expect("invalid metadata"),
        event_type: event.event_type(),
        event_version: event.event_version(),
    }
}

pub fn to_serialized_command_envelope<C: DomainCommand>(
    app_id: impl Into<String>,
    aggregate_id: impl Into<String>,
    command: C,
    metadata: HashMap<String, String>,
) -> SerializedCommandEnvelope {
    SerializedCommandEnvelope {
        aggregate_id: aggregate_id.into(),
        app_id: app_id.into(),
        cmd_name: command.command_name(),
        cmd_payload: command.command_payload(),
        metadata,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppPermission {
    pub name: String,
    pub description: String,
    pub app: String, // app
    pub cmds: Vec<String>,
    pub events: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub permissions: Vec<AppPermission>,
}

#[derive(Debug)]
pub enum RunnableError {
    Unknown,
}

#[async_trait]
pub trait AppRunnable: Send {
    fn get_app_info(&self) -> &AppInfo;

    fn snapshot(&self) -> Value;

    async fn apply(&mut self, event: CommittedEventEnvelope) -> Result<(), RunnableError>;

    async fn run_command(
        &self,
        command: SerializedCommandEnvelope,
    ) -> Result<Vec<SerializedEventEnvelope>, RunnableError>;
}
