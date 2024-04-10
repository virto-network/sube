use async_std::test;
use async_trait::async_trait;
use virto_sdk::prelude::*;
use virto_sdk::*;
use crate::fixtures::app_mock::*;

#[async_std::test]
async fn run_command_and_take_snapshot() {
    let info = mock_app_info();

    let app_factory = AppFactory::<MockApp, ServiceConfig>::new(
        info,
        ServiceConfig { url: "http".into() }, // container reference
    );

    let mut app = app_factory.run(None);

    let events = app
        .run_command(to_envelop_cmd(MockAppCmd::B))
        .await
        .expect("hello");

    for (seq, e) in events.into_iter().enumerate() {
        app.apply(to_committed_event(seq, e)).await;
    }

    let snapshot = app.snapshot();
    let state: MockApp = serde_json::from_value(snapshot).expect("It must increase");
    assert!(state.sum == 11);
}
