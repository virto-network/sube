use ::std::sync::Arc;
use async_std::test;
use async_trait::async_trait;
use futures::{ channel::mpsc::{channel, Receiver, Sender}, StreamExt};
use futures::SinkExt;
use virto_sdk::prelude::*;
use virto_sdk::*;

use crate::fixtures::app_mock::*;

#[async_std::test]
async fn spawn_process_and_communicate() {
    let info = mock_app_info();

    let appLoader: Box<dyn AppLoader> = Box::new(AppFactory::<MockApp, ServiceConfig>::new(
        info,
        ServiceConfig { url: "http".into() },
    ));

    let mut pool = Pool::new();
    let (mut tx, mut rx) = pool
        .spawn(None, &appLoader)
        .expect("it must spawn a process");


    tx.send(Instruction::Cmd(to_envelop_cmd(MockAppCmd::B)))
        .await;

    tx.send(Instruction::Snapshot("agg_id".to_string()))
        .await;

    
    while let Some(r) = rx.next().await {
      if let Response::Snapshot { agg_id, app_id, state } = r {
        assert_eq!(agg_id, "agg_id");
        assert_eq!(agg_id, "foo");
        break;
      }
    }

}
