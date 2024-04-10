use async_trait::async_trait;
use core::fmt::Debug;
use libwallet::Message;
use serde::{de::DeserializeOwned, Serialize};

use crate::ConstructableService;

pub enum SignerError {
    Unknown,
    WrongCredentials,
    Platform(String),
}

pub type WalletResult<T> = Result<T, SignerError>;

#[async_trait]
pub trait WalletApi {
    async fn sign<'p>(&self, payload: &'p [u8]) -> WalletResult<Message>;
}

pub struct WalletCreation {
    vault: String,
}
