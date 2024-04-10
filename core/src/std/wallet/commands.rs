use super::Message;
use crate::prelude::*;

#[derive(Deserialize, Debug, Serialize)]
pub enum WalletCommand {
    AddMessageToSign(Message),
    Sign(),
}
