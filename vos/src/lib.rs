use std::{error::Error, pin::Pin};

use futures_util::{SinkExt, StreamExt};
use io::{Auth, Input, InputStream, Output, OutputSink};
use matrix_sdk::{ruma::UserId, Client as MxClient};

mod io;
#[cfg(target_arch = "wasm32")]
mod js_worker;
#[cfg(target_arch = "wasm32")]
use js_worker::setup_io;

#[cfg(not(target_arch = "wasm32"))]
fn setup_io() -> (impl InputStream, impl OutputSink) {
    // TODO dummy
    (
        futures_util::stream::once(async { Input::Prompt("hello".into()) }),
        futures_util::sink::unfold(Output::Empty, |o, _| async { Ok(o) }),
    )
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let (input, out) = setup_io();
    let sh = Shell::new(input);
    sh.process_input_stream(Box::pin(out)).await;
    Ok(())
}

struct Shell {
    mx: Option<MxClient>,
    in_stream: Pin<Box<dyn InputStream>>,
}

impl Shell {
    pub fn new(input: impl io::InputStream) -> Self {
        Self {
            mx: None,
            in_stream: Box::pin(input),
        }
    }

    pub async fn process_input_stream(mut self, mut out: Pin<Box<dyn OutputSink>>) {
        while let Some(input) = self.in_stream.next().await {
            out.send(self.handle_input(input).await)
                .await
                .unwrap_or_else(|_| {
                    log::warn!("failed sending output");
                });
        }
    }

    async fn handle_input(&mut self, input: Input) -> io::Result {
        if !self.mx.as_ref().is_some_and(|m| m.logged_in()) {
            return Ok(Output::WaitingAuth([0; 32]));
        }
        Ok(match input {
            Input::Empty => todo!(),
            Input::Auth(user, auth) => self.connect(&user, auth).await?,
            Input::Prompt(_) => todo!(),
            Input::Open(_) => todo!(),
            Input::Answer(_) => todo!(),
            Input::Data(_) => todo!(),
        })
    }

    pub async fn connect(&mut self, user: &str, credentials: Auth) -> io::Result {
        let mid = UserId::parse(user).map_err(|_| ())?;
        let mx = MxClient::new(mid.server_name().as_str().try_into().unwrap())
            .await
            .map_err(|_| ())?;

        let auth = mx.matrix_auth();
        let flows = auth.get_login_types().await.map_err(|_| ())?.flows;
        log::info!("{:?}", flows);

        match credentials {
            Auth::Pwd { user: _, pwd: _ } => todo!(),
            Auth::Authenticator(_) => todo!(),
        }

        self.mx.replace(mx);
        Ok(Output::Empty)
    }
}
