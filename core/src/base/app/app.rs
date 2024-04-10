use super::types::{
    AppInfo, AppRunnable, DomainEvent, EventEnvelope, SerializedCommandEnvelope, StateMachine,
};

use crate::{prelude::*, CommittedEventEnvelope, ConstructableService, SerializedEventEnvelope};
use crate::{to_serialized_event_envelope, RunnableError};

pub struct AppFactory<S, Args>
where
    S: StateMachine + Sync + Send,
    S::Services: ConstructableService<Args = Args, Service = S::Services>,
    Args: Clone,
{
    args: Args,
    app_info: AppInfo,
    state_machine: PhantomData<S>,
}

pub trait AppLoader<'r>: Send + Sync {
    fn app_info(&self) -> &AppInfo;
    fn run(&self, state: Option<Value>) -> Box<dyn AppRunnable + 'r>;
}

impl<S, Args> AppFactory<S, Args>
where
    S: StateMachine + Sync + Send ,
    S::Services: ConstructableService<Args = Args, Service = S::Services>,
    Args: Clone + Send + Sync,
{
    pub fn new(app_info: AppInfo, args: Args) -> Self {
        Self {
            app_info,
            args,
            state_machine: Default::default(),
        }
    }
}

impl<'r, S, Args> AppLoader<'r> for AppFactory<S, Args>
where
    S: StateMachine + Sync + Send + 'r,
    S::Services: ConstructableService<Args = Args, Service = S::Services>,
    Args: Clone + Send + Sync,
{
    fn app_info(&self) -> &AppInfo {
        &self.app_info
    }

    fn run(&self, state: Option<Value>) -> Box<dyn AppRunnable + 'r> {
        let state_machine: S = match state {
            Some(value) => serde_json::from_value(value).expect("Can not serialize State"),
            None => S::default(),
        };

        let service = S::Services::new(self.args.clone());

        Box::new(App::new(self.app_info.clone(), state_machine, service))
    }
}

pub struct App<S>
where
    S: StateMachine + Send + Sync,
{
    app_info: AppInfo,
    services: S::Services,
    state_machine: S,
}

impl<S> App<S> where S:  StateMachine + Send + Sync {
    fn new(app_info: AppInfo, state: S, services: S::Services) -> Self {
        Self {
            app_info,
            services,
            state_machine: state,
        }
    }
}

#[async_trait]
impl<S> AppRunnable for App<S>
where
    S: StateMachine + Send + Sync,
{
    fn snapshot(&self) -> Value {
        serde_json::to_value(&self.state_machine).expect("It must be a serializable state_machine")
    }

    fn get_app_info(&self) -> &AppInfo {
        &self.app_info
    }

    async fn apply(&mut self, event: CommittedEventEnvelope) -> Result<(), RunnableError> {
        let event: S::Event =
            serde_json::from_value(event.payload).map_err(|_| RunnableError::Unknown)?;
        println!("{:?}", event);

        self.state_machine.apply(event);
        Ok(())
    }

    async fn run_command(
        &self,
        command: SerializedCommandEnvelope,
    ) -> Result<Vec<SerializedEventEnvelope>, RunnableError> {
        let cmd: S::Command =
            serde_json::from_value(command.cmd_payload).map_err(|_| RunnableError::Unknown)?;

        let events = self
            .state_machine
            .handle(cmd, &self.services)
            .await
            .map_err(|_| RunnableError::Unknown)?;

        Ok(events
            .into_iter()
            .map(|e| {
                to_serialized_event_envelope::<S::Event>(
                    &self.app_info.id,
                    &command.aggregate_id,
                    &e,
                    &command.metadata,
                )
            })
            .collect())
    }
}
