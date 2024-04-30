#![feature(async_closure)]

use jsonrpc::error;
use libwallet::{self, vault, Account, Signature};
use rand_core::OsRng;
use serde_json::json;
use std::env;
use sube::builder::SubeBuilder;

type Wallet = libwallet::Wallet<vault::Simple<String>>;
use anyhow::{anyhow, Result};

#[async_std::main]
async fn main() -> Result<()> {
    let phrase = env::args().skip(1).collect::<Vec<_>>().join(" ");

    let (vault, phrase) = if phrase.is_empty() {
        vault::Simple::generate_with_phrase(&mut rand_core::OsRng)
    } else {
        let phrase: libwallet::Mnemonic = phrase.parse().expect("Invalid phrase");
        (vault::Simple::from_phrase(&phrase), phrase)
    };

    let mut wallet = Wallet::new(vault);

    wallet.unlock(None, None).await.map_err(|e| anyhow!("error"))?;

    let account = wallet.default_account().unwrap();
    
    let response = SubeBuilder::default()
        .with_url("wss://rococo-rpc.polkadot.io/balances/transfer")
        .with_signer(async |message: &[u8]|  Ok(wallet.sign(message).await.expect("it must sign").as_bytes()) )
        .with_sender(account.public().as_ref())
        .with_body(json!({
            "dest": {
                "Id": account.public().as_ref()
            },
            "value": 100000
        }))
        .await
        .map_err(|err| anyhow!(format!("Error {:?}", err)))?;

    Ok(())
}
