use crate::ConstructableService;

use super::super::types::StateMachine;

pub struct StateMachineResultValidator<S>
where
    S: StateMachine,
{
    result: Result<Vec<S::Event>, S::Error>,
}

impl<S: StateMachine> StateMachineResultValidator<S> {
    pub fn then_expect_events(self, expected_events: Vec<S::Event>) {
        let events = match self.result {
            Ok(expected_events) => expected_events,
            Err(err) => {
                panic!("expected success, received aggregate error: '{}'", err);
            }
        };
        assert_eq!(events, expected_events);
    }

    pub fn then_expect_error_message(self, error_message: &str) {
        match self.result {
            Ok(events) => {
                panic!("expected error, received events: '{:?}'", events);
            }
            Err(err) => assert_eq!(err.to_string(), error_message.to_string()),
        };
    }

    pub fn inspect_result(self) -> Result<Vec<S::Event>, S::Error> {
        self.result
    }

    pub(crate) fn new(result: Result<Vec<S::Event>, S::Error>) -> Self {
        Self { result }
    }
}
impl<A> StateMachineResultValidator<A>
where
    A: StateMachine,
    A::Error: PartialEq,
{
    pub fn then_expect_error(self, expected_error: A::Error) {
        match self.result {
            Ok(events) => {
                panic!("expected error, received events: '{:?}'", events);
            }
            Err(err) => {
                assert_eq!(err, expected_error);
            }
        }
    }
}
