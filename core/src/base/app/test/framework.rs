use std::env::Args;

use async_trait::async_trait;

use crate::ConstructableService;

use super::super::types::StateMachine;
use super::executor::StateMachineTestExecutor;

/// A framework for rigorously testing the aggregate logic, one of the *most important*
/// parts of any DDD system.
pub struct TestFramework<S: StateMachine> {
    service: S::Services,
}

impl<S: StateMachine> TestFramework<S> {
    /// Create a test framework using the provided service.
    pub fn with(service: S::Services) -> Self {
        Self { service }
    }
}

impl<S> TestFramework<S>
where
    S: StateMachine,
{
    #[must_use]
    pub fn given_no_previous_events(self) -> StateMachineTestExecutor<S> {
        StateMachineTestExecutor::new(Vec::new(), self.service)
    }

    #[must_use]
    pub fn given(self, events: Vec<S::Event>) -> StateMachineTestExecutor<S> {
        StateMachineTestExecutor::new(events, self.service)
    }
}
