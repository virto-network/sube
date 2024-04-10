
use virto_sdk::*;
use virto_sdk::prelude::*;
use async_trait::async_trait;
use ::std::fmt;

#[derive(Deserialize, Serialize, Debug)]
pub enum MockAppCmd {
    A,
    B,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum MockAppEvent {
    A,
    B(usize),
}

#[derive(Debug)]
pub enum MockTestError {
    A,
    B,
}

impl core::fmt::Display for MockTestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "Error::A"),
            Self::B => write!(f, "Error::B"),
        }
    }
}

impl ::std::error::Error for MockTestError {}

impl DomainEvent for MockAppEvent {
    fn event_type(&self) -> String {
        match self {
            MockAppEvent::A => "A".into(),
            MockAppEvent::B(_) => "B".into(),
        }
    }

    fn event_version(&self) -> String {
        "0.0.1".into()
    }
}

impl DomainCommand for MockAppCmd {
    fn command_name(&self) -> String {
        match self {
            MockAppCmd::A => "A".into(),
            MockAppCmd::B => "B".into(),
        }
    }

    fn command_payload(&self) -> Value {
        serde_json::to_value(self).expect("Can not Serialize command")
    }
}

#[derive(Serialize, Deserialize)]
pub struct MockApp {
    pub sum: usize,
}

impl Default for MockApp {
    fn default() -> Self {
        Self { sum: 0 }
    }
}

pub trait MockService: Sync + Send {
    fn sign(&self) -> usize;
}

#[derive(Deserialize, Clone)]
pub struct ServiceConfig {
    pub url: String,
}

#[derive(Default)]
pub struct Service {
    pub url: String,
}

impl Service {
    fn new(args: ServiceConfig) -> Self {
        Service { url: args.url }
    }
}

impl MockService for Service {
    fn sign(&self) -> usize {
        10
    }
}

pub fn to_envelop_cmd(cmd: MockAppCmd) -> SerializedCommandEnvelope {
    to_serialized_command_envelope("0.0.1", "david", cmd, HashMap::new())
}

impl ConstructableService for Box<dyn MockService> {
    type Args = ServiceConfig;
    type Service = Box<dyn MockService>;

    fn new(args: Self::Args) -> Self::Service {
        Box::new(Service::new(args))
    }
}

pub fn mock_app_info() -> AppInfo {
    AppInfo {
        author: "foo".into(),
        description: "foo".into(),
        id: "foo".into(),
        name: "hello".into(),
        permissions: vec![],
        version: "0.0.1".into(),
    }
}

pub fn to_committed_event(
    sequence: usize,
    event: SerializedEventEnvelope,
) -> CommittedEventEnvelope {
    CommittedEventEnvelope {
        aggregate_id: event.aggregate_id,
        app_id: event.app_id,
        event_type: event.event_type,
        event_version: event.event_version,
        metadata: event.metadata,
        payload: event.payload,
        sequence,
    }
}

#[async_trait]
impl StateMachine for MockApp {
    type Command = MockAppCmd;
    type Event = MockAppEvent;
    type Error = MockTestError;
    type Services = Box<dyn MockService + 'static>;

    async fn handle(
        &self,
        command: Self::Command,
        ctx: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            MockAppCmd::A => Ok(vec![MockAppEvent::A]),
            MockAppCmd::B => Ok(vec![MockAppEvent::A, MockAppEvent::B(ctx.sign())]),
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            MockAppEvent::A => {
                self.sum += 1;
            }
            MockAppEvent::B(u) => {
                self.sum += u;
            }
        }
    }
}
