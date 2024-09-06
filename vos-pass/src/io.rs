use futures_util::{Sink, Stream};
use serde::{Deserialize, Serialize};

pub type Result = core::result::Result<Output, ()>;
pub type Id = u32;

#[derive(Debug, Deserialize)]
pub enum Input {
    Empty,
    Prompt(String),
    Auth(String, Auth),
    Open(String),
    Answer(String),
    Data(Vec<u8>),
}

#[derive(Debug, Deserialize)]
pub enum Auth {
    Pwd { user: String, pwd: String },
    Credential {},
}

type Challenge = [u8; 32];

#[derive(Debug, Serialize)]
pub struct Message {
    id: Id,
    ts: u32,
    msg: String,
}

#[derive(Debug, Serialize)]
pub enum Output {
    Empty,
    Busy,
    Msg(Message),
    MsgUpdate { id: Id, msg: String },
    WaitingAuth(Challenge),
    WaitingInput(String),
    WaitintData,
}

pub trait InputStream: Stream<Item = Input> + 'static {}
impl<T: Stream<Item = Input> + 'static> InputStream for T {}

pub trait OutputSink: Sink<Result, Error = ()> {}
impl<T: Sink<Result, Error = ()>> OutputSink for T {}
