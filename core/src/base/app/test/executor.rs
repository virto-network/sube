use crate::ConstructableService;

use super::super::types::StateMachine;
use super::validator::StateMachineResultValidator;

use tokio::main;

pub struct StateMachineTestExecutor<A>
where
    A: StateMachine,
{
    events: Vec<A::Event>,
    service: A::Services,
}

impl<S> StateMachineTestExecutor<S>
where
    S: StateMachine,
{
    pub fn when(self, command: S::Command) -> StateMachineResultValidator<S> {
        let result = when::<S>(self.events, command, self.service);
        StateMachineResultValidator::new(result)
    }

    pub async fn when_async(self, command: S::Command) -> StateMachineResultValidator<S> {
        let mut aggregate = S::default();
        for event in self.events {
            aggregate.apply(event);
        }
        let result = aggregate
            .handle(command, &self.service)
            .await
            .map(|x| x.into_iter().collect());
        StateMachineResultValidator::<S>::new(result)
    }

    #[must_use]
    pub fn and(self, new_events: Vec<S::Event>) -> Self {
        let mut events = self.events;
        events.extend(new_events);
        let service = self.service;
        StateMachineTestExecutor { events, service }
    }

    pub(crate) fn new(events: Vec<S::Event>, service: S::Services) -> Self {
        Self { events, service }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn when<S: StateMachine>(
    events: Vec<S::Event>,
    command: S::Command,
    service: S::Services,
) -> Result<Vec<S::Event>, S::Error> {
    let mut aggregate = S::default();
    for event in events {
        aggregate.apply(event);
    }
    aggregate.handle(command, &service).await
}
