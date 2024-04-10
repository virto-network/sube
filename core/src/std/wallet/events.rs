use super::Message;
use crate::DomainEvent;
use crate::utils::prelude::*;

#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub enum WalletEvent {
    AddedMessageToSign(Message),
    Signed(Vec<(Message, Message)>),
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub enum WalletError {
    Unknown,
    NoMessagesToSign,
}

impl core::fmt::Display for WalletError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::NoMessagesToSign => write!(f, "NoMessagesToSign"),
        }
    }
}

impl core::error::Error for WalletError {}

impl DomainEvent for WalletEvent {
    fn event_type(&self) -> String {
        match self {
            Self::AddedMessageToSign(_) => "AddedMessageToSign".to_string(),
            Self::Signed(..) => "Signed".to_string(),
        }
    }
    fn event_version(&self) -> String {
        "0.1.0".to_string()
    }
}
