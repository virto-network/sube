use crate::{
    AppLoader, AppRegistry, DomainCommand, Instruction, Pool, Process, Response,
    SerializedCommandEnvelope, StateManager, Store, StoreState, TxInstruction,
};

use async_std::task::spawn;
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    SinkExt, StreamExt,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub struct Supervisor<'r: 'static, R: AppRegistry, S: StateManager> {
    registry: &'r R,
    pool: Pool,
    state_manager: &'r S,
}

#[derive(Debug)]
pub enum SupervisorError {
    Unknown,
    AppNotInstalled,
    ErrorAtSpawn,
    CantSaveState,
    CantCommit,
}

type SupervisorResult<T> = Result<T, SupervisorError>;

#[derive(Serialize)]
pub struct CommandRequest<T: DomainCommand + Serialize> {
    to: String,
    cmd: T,
    metadata: HashMap<String, String>,
}

impl<'r: 'static, R: AppRegistry, S: StateManager> Supervisor<'r, R, S> {
    fn new(registry: &'r R, state_manager: &'r S) -> Self {
        Self {
            registry,
            state_manager,
            pool: Pool::default(),
        }
    }

    fn check_permission_cmd(
        state: &StoreState,
        from_app_id: &str,
        to_app_id: &str,
        cmd: &str,
    ) -> bool {
        if let Some(metadata) = state.get(to_app_id) {
            if let Some(permission) = metadata
                .app_info
                .permissions
                .iter()
                .find(|p| p.app == from_app_id)
            {
                return permission.cmds.iter().any(|p| p == cmd);
            }
        }

        false
    }

    async fn try_spawn_and_listen(
        &mut self,
        app_id: &str,
        agg_id: &str,
    ) -> SupervisorResult<TxInstruction> {
        let loader = self
            .registry
            .get_loader(&app_id)
            .await
            .ok_or(SupervisorError::ErrorAtSpawn)?;

        let state = self
            .state_manager
            .load_state(&app_id, &agg_id)
            .await
            .map_err(|_| SupervisorError::ErrorAtSpawn)?;

        match self.pool.get_or_spawn(&app_id, state, loader).await {
            Process::Running(sender) => Ok(sender.clone()),
            Process::Spawned((tx, mut rx)) => {
                let mut tx_routine = tx.clone();
                let manager: &'r dyn StateManager = self.state_manager;

                spawn(async move {
                    // it keeps order sequence
                    while let Some(response) = rx.next().await {
                        match response {
                            Response::Events {
                                app_id,
                                agg_id,
                                events,
                                metadata,
                            } => {
                                let committed_events = 
                                    manager.commit(&app_id, events, metadata)
                                    .await
                                    .map_err(|_| SupervisorError::CantCommit)?;

                                tx_routine
                                    .send(Instruction::CommitEvent(committed_events))
                                    .await
                                    .map_err(|_| SupervisorError::Unknown)?;

                                tx_routine
                                    .send(Instruction::Snapshot(agg_id))
                                    .await
                                    .map_err(|_| SupervisorError::Unknown)?;
                            }
                            Response::Snapshot {
                                app_id,
                                agg_id,
                                state,
                            } => manager
                                .save_state(&app_id, &agg_id, state)
                                .await
                                .map_err(|_| SupervisorError::CantSaveState)?,
                            Response::Exit => {}
                        }
                    }
                    Ok::<(), SupervisorError>(())
                });

                Ok(tx.clone())
            }
            Process::Error(err) => Err(SupervisorError::Unknown),
        }
    }

    pub async fn exec<T: DomainCommand + Serialize>(
        &mut self,
        who: &str,
        cmd: CommandRequest<T>,
    ) -> SupervisorResult<()> {
        let (app_id, agg_id) = match cmd.to.split('#').collect::<Vec<&str>>().as_slice() {
            [first, last] => (*first, *last),
            _ => return Err(SupervisorError::Unknown),
        };

        let mut tx = self.try_spawn_and_listen(&app_id, &agg_id).await?;

        tx.send(Instruction::Cmd(SerializedCommandEnvelope {
            aggregate_id: agg_id.to_string(),
            app_id: app_id.to_string(),
            cmd_name: cmd.cmd.command_name(),
            cmd_payload: cmd.cmd.command_payload(),
            metadata: cmd.metadata.clone(),
        }))
        .await
        .map_err(|_| SupervisorError::Unknown)?;

        Ok(())
    }
}
