use async_trait::async_trait;
use jsonrpc::{
    error::{standard_error, StandardError},
    serde_json::{to_string, value::to_raw_value},
};
use std::{convert::TryInto, fmt};
pub use surf::Url;

use crate::rpc::{self, Rpc, RpcResult};

#[derive(Debug)]
pub struct Backend(Url);

impl Backend {
    pub fn new<U>(url: U) -> Self
    where
        U: TryInto<Url>,
        <U as TryInto<Url>>::Error: fmt::Debug,
    {
        Backend(url.try_into().expect("Url"))
    }
}

#[async_trait]
impl Rpc for Backend {
    /// HTTP based JSONRpc request expecting an hex encoded result
    async fn rpc(&self, method: &str, params: &[&str]) -> RpcResult {
        log::info!("RPC `{}` to {}", method, &self.0);
        let res = surf::post(&self.0)
            .content_type("application/json")
            .body(
                to_string(&rpc::Request {
                    id: 1.into(),
                    jsonrpc: Some("2.0"),
                    method,
                    params: &Self::convert_params(params),
                })
                .unwrap(),
            )
            .await
            .map_err(|err| rpc::Error::Transport(err.into_inner().into()))?
            .body_json::<rpc::Response>()
            .await
            .map_err(|err| {
                standard_error(
                    StandardError::ParseError,
                    Some(to_raw_value(&err.to_string()).unwrap()),
                )
            })?
            .result::<String>()?;

        log::debug!("RPC Response: {}...", &res[..res.len().min(20)]);
        // assume the response is a hex encoded string starting with "0x"
        let response = hex::decode(&res[2..])
            .map_err(|_err| standard_error(StandardError::InternalError, None))?;
        Ok(response)
    }
}
