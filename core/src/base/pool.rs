use core::marker::PhantomData;
use std::collections::HashMap;
use std::hash::Hash;

#[cfg(feature = "no_std")]
use alloc::sync::Arc;
use matrix_sdk::ruma::api::Metadata;

#[cfg(not(feature = "no_std"))]
use std::rc::Rc;

use crate::{
    AppInfo, AppLoader, AppRunnable, CommittedEventEnvelope, SerializedCommandEnvelope,
    SerializedEventEnvelope,
};
use async_std::stream::StreamExt;
use async_std::task::spawn;

use async_trait::async_trait;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::stream::Next;
use futures::{Future, SinkExt, Stream};
use serde_json::Value;

#[derive(Debug)]
pub enum Instruction {
    Cmd(SerializedCommandEnvelope),
    CommitEvent(Vec<CommittedEventEnvelope>),
    Snapshot(String), // id
    Kill,
}

#[derive(Debug)]
pub enum Response {
    Events {
        app_id: String,
        agg_id: String,
        events: Vec<SerializedEventEnvelope>,
        metadata: HashMap<String, String>,
    },
    Snapshot {
        app_id: String,
        agg_id: String,
        state: Value,
    },
    Exit,
}

pub type TxInstruction = Sender<Instruction>;
pub type RxResponse = Receiver<Response>;

#[cfg(feature = "single_thread")]
pub struct Pool {
    pub apps: HashMap<String, TxInstruction>,
}

#[derive(Debug)]
pub enum ProcessError {
    NotRunning,
    AlreadyRunning,
    Unknown,
}

pub enum Process {
    Spawned((TxInstruction, RxResponse)),
    Running(TxInstruction),
    Error(String),
}

pub type ProcessResult<T> = Result<T, ProcessError>;

impl Default for Pool {
    fn default() -> Self {
        Pool::new()
    }
}

impl Pool {
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
        }
    }

    pub async fn get_or_spawn<'r: 'static>(
        &mut self,
        app_id: &str,
        state: Option<Value>,
        loader: &Box<dyn AppLoader<'r>>,
    ) -> Process {
        match self.apps.get(app_id) {
            Some(tx) => Process::Running(tx.clone()),
            None => match self.spawn::<'r>(state, loader) {
                Ok((tx, rx)) => {
                    self.apps.insert(app_id.into(), tx.clone());
                    Process::Spawned((tx, rx))
                }
                Err(_) => Process::Error("Cant spawn process".into()),
            },
        }
    }

    pub fn spawn<'r: 'static>(
        &mut self,
        state: Option<Value>,
        loader: &Box<dyn AppLoader<'r>>,
    ) -> ProcessResult<(TxInstruction, RxResponse)> {
        let (tx, mut rx) = channel::<Instruction>(100);
        let (mut tx_response, mut rx_response) = channel::<Response>(100);

        let app_info = loader.app_info();

        if let Some(_) = self.apps.get(&app_info.id) {
            return Err(ProcessError::AlreadyRunning);
        }

        let mut app = loader.run(state);

        spawn(async move {
            while let Some(instruction) = rx.next().await {
                match instruction {
                    Instruction::Cmd(command) => {
                        let events = app
                            .run_command(command.clone())
                            .await
                            .map_err(|_| ProcessError::Unknown)?;

                        tx_response
                            .send(Response::Events {
                                agg_id: command.aggregate_id,
                                app_id: command.app_id,
                                events: events,
                                metadata: command.metadata,
                            })
                            .await
                            .map_err(|_| ProcessError::Unknown)?;
                    }
                    Instruction::CommitEvent(event) => {
                        for i in event {
                            app.apply(i).await.map_err(|_| ProcessError::Unknown)?;
                        }
                    }
                    Instruction::Snapshot(agg_id) => {
                        let state = app.snapshot();
                        let app_id = app.get_app_info().id.to_string();
                        tx_response
                            .send(Response::Snapshot {
                                app_id,
                                agg_id,
                                state,
                            })
                            .await
                            .map_err(|_| ProcessError::Unknown)?;
                    }
                    Instruction::Kill => {
                        drop(app);
                        tx_response.send(Response::Exit);
                        return Err(ProcessError::Unknown);
                    }
                }
            }

            Ok::<(), ProcessError>(())
        });

        Ok((tx.clone(), rx_response))
    }
}
