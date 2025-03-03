#![cfg_attr(not(feature = "std"), no_std)]
#![allow(async_fn_in_trait)]
/*!
Sube is a lightweight blockchain client to query and submit extrinsics
to Substrate based blockchains.
It supports multiple backends and uses the chain's type information
to automatically encode/decode data into human-readable formats like JSON.

TODO: rewrite docs for sube 1.0
*/

#[cfg(not(any(feature = "v14")))]
compile_error!("Enable one of the metadata versions");

#[macro_use]
extern crate alloc;

pub use codec;
use codec::Encode;
pub use core::fmt::Display;
use core::iter::Empty;

pub use frame_metadata::RuntimeMetadataPrefixed;
pub use signer::{Bytes, Signer, SignerFn};

pub use meta::Metadata;
#[cfg(feature = "v14")]
pub use scales::{Serializer, Value};

use codec::Compact;
use core::fmt;
use hasher::hash;
// use meta::Meta;
use meta_ext::{self as meta, Meta as _};
use meta_ext::{KeyValue, StorageKey};
use prelude::*;
#[cfg(feature = "v14")]
use scale_info::PortableRegistry;
use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value as JsonValue};

use crate::util::to_camel;

mod prelude {
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
}

/// Surf based backend
#[cfg(any(feature = "http", feature = "http-web"))]
pub mod http;
/// Tungstenite based backend
#[cfg(feature = "ws")]
pub mod ws;

pub mod builder;
pub use builder::SubeBuilder;
mod hasher;
pub mod meta_ext;
mod signer;

#[cfg(any(feature = "http", feature = "http-web", feature = "ws"))]
pub mod rpc;
pub mod util;

/// The batteries included way to query or submit extrinsics to a Substrate based blockchain
///
/// Returns a builder that implments `IntoFuture` so it can be `.await`ed on.
pub fn sube(url: &str) -> builder::SubeBuilder<(), ()> {
    builder::SubeBuilder::default().with_url(url)
}

pub type Result<T> = core::result::Result<T, Error>;

async fn query<'m>(
    chain: &impl Backend,
    meta: &'m Metadata,
    path: &str,
    block: Option<u32>,
) -> Result<Response<'m>> {
    let (pallet, item_or_call, mut keys) = parse_uri(path).ok_or(Error::BadInput)?;
    let pallet = meta
        .pallet_by_name(&pallet)
        .ok_or_else(|| Error::PalletNotFound(pallet))?;

    if item_or_call == "_constants" {
        let const_name = keys.pop().ok_or_else(|| Error::MissingConstantName)?;
        let const_meta = pallet
            .constants
            .iter()
            .find(|c| c.name == const_name)
            .ok_or_else(|| Error::ConstantNotFound(const_name))?;

        return Ok(Response::Value(Value::new(
            const_meta.value.clone(),
            const_meta.ty.id,
            &meta.types,
        )));
    }

    if let Ok(key_res) = StorageKey::build_with_registry(&meta.types, pallet, &item_or_call, &keys)
    {
        if !key_res.is_partial() {
            let res = chain.get_storage_item(key_res.key(), block).await?;

            let value = res.map_or(Response::None, |res| {
                Response::Value(Value::new(res, key_res.ty, &meta.types))
            });

            return Ok(value);
        }

        let res = chain.get_keys_paged(key_res.key(), 1000, None).await?;
        let result = chain.get_storage_items(res, block).await?;

        let value = result
            .into_iter()
            .map(|(key, data)| {
                let key = &key[(key_res.pallet.len() + key_res.call.len())..];
                let mut offset = 16; // TODO it depends on the hasher used to encode the key, then the size could change
                let keys = key_res
                    .args
                    .iter()
                    .map(|arg| match arg {
                        KeyValue::Empty(type_id) | KeyValue::Value((type_id, _, _, _)) => {
                            let hashed = &key[offset..];
                            let value = Value::new(hashed.to_vec(), *type_id, &meta.types);
                            offset += value.size() + 16;
                            value
                        }
                    })
                    .collect::<Vec<Value<'m>>>();

                let value = data.map_or(None, |data| {
                    Some(Value::new(data.to_vec(), key_res.ty, &meta.types))
                });
                (keys, value)
            })
            .collect::<Vec<_>>();

        Ok(Response::ValueSet(value))
    } else {
        Err(Error::ChainUnavailable)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtrinsicBody<Body> {
    pub nonce: Option<u64>,
    pub body: Body,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct AccountInfo {
    nonce: u64,
    consumers: u64,
    providers: u64,
    sufficients: u64,
    data: Data,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Data {
    free: u128,
    reserved: u128,
    frozen: u128,
    flags: u128,
}

async fn submit<'m, V>(
    chain: impl Backend,
    meta: &'m Metadata,
    path: &str,
    tx_data: ExtrinsicBody<V>,
    signer: impl Signer,
) -> Result<Response<'m>>
where
    V: serde::Serialize + core::fmt::Debug,
{
    let (pallet, item_or_call, _keys) = parse_uri(path).ok_or(Error::BadInput)?;
    let pallet = meta
        .pallet_by_name(&pallet)
        .ok_or_else(|| Error::PalletNotFound(pallet))?;
    let calls_ty = pallet.calls.as_ref().ok_or(Error::CallNotFound)?.ty.id;

    log::debug!("calls_ty: {:?}", calls_ty);

    let type_registry = &meta.types;

    let mut encoded_call = vec![pallet.index];

    log::debug!("tx_data: {:?}", tx_data);
    let json = &json!({
        &item_or_call.to_lowercase(): &tx_data.body
    });
    log::debug!("json_body: {:?}", &json);

    let call_data = scales::to_vec_with_info(&json, (type_registry, calls_ty).into())
        .map_err(|e| Error::Encode(e.to_string()))?;

    encoded_call.extend(&call_data);

    let from_account = signer.account();
    log::debug!("from_account: {:?}", hex::encode(from_account.as_ref()));

    let extra_params = {
        // ImmortalEra
        let era = 0u8;

        // Impl. Note: in a real-world use case, you should store your account's nonce somewhere else
        let nonce = {
            if let Some(nonce) = tx_data.nonce {
                Ok(nonce)
            } else {
                let response = query(
                    &chain,
                    meta,
                    &format!("system/account/0x{}", hex::encode(from_account.as_ref())),
                    None,
                )
                .await?;

                match response {
                    Response::Value(value) => {
                        let str = serde_json::to_string(&value).expect("wrong account info");
                        let account_info: AccountInfo =
                            serde_json::from_str(&str).expect("it must serialize");
                        Ok(account_info.nonce)
                    }
                    Response::None => {
                        log::warn!("account not found");
                        Ok(0)
                    }
                    _ => Err(Error::AccountNotFound),
                }
            }
        }?;

        let tip: u128 = 0;

        [
            vec![era],
            Compact(nonce).encode(),
            Compact(tip).encode(),
            vec![0x00u8], // chain extension for kreivo
        ]
        .concat()
    };

    let additional_params = {
        // Error: Still failing to deserialize the const
        let metadata = meta;

        let mut constants = metadata
            .pallet_by_name("System")
            .ok_or(Error::PalletNotFound(String::from("System")))?
            .constants
            .clone()
            .into_iter();

        let data = constants
            .find(|c| c.name == "Version")
            .ok_or(Error::ConstantNotFound("System_Version".into()))?;

        let chain_value: JsonValue = Value::new(data.value, data.ty.id, &meta.types).into();

        let iter = chain_value
            .as_object()
            .ok_or(Error::ConstantNotFound("System_Version".into()))?;

        let transaction_version = iter.get("transaction_version").ok_or(Error::Mapping(
            "System_Version.transaction_version not found in transaction version".into(),
        ))?;

        let spec_version = iter.get("spec_version").ok_or(Error::Mapping(
            "System_Version.spec_version not found in transaction version".into(),
        ))?;
        // chain_version

        let spec_version = spec_version.as_u64().ok_or(Error::Mapping(
            "System_Version.spec_version is not a number".into(),
        ))? as u32;

        let transaction_version = transaction_version.as_u64().ok_or(Error::Mapping(
            "System_Version.transaction_version is not a number".into(),
        ))? as u32;

        let genesis_block: Vec<u8> = chain.block_info(Some(0u32)).await?.into();

        [
            spec_version.to_le_bytes().to_vec(),
            transaction_version.to_le_bytes().to_vec(),
            genesis_block.clone(),
            genesis_block.clone(),
        ]
        .concat()
    };

    let signature_payload = [
        encoded_call.clone(),
        extra_params.clone(),
        additional_params.clone(),
    ]
    .concat();

    let payload = if signature_payload.len() > 256 {
        hash(&meta::Hasher::Blake2_256, &signature_payload[..])
    } else {
        signature_payload
    };

    let signature = signer.sign(payload).await?;

    let extrinsic_call = {
        let encoded_inner = [
            // header: "is signed" (1 byte) + transaction protocol version (7 bytes)
            vec![0b10000000 + 4u8],
            // signer
            vec![0x00],
            from_account.as_ref().to_vec(),
            // signature
            [vec![0x01], signature.as_ref().to_vec()].concat(),
            // extra
            extra_params,
            // call data
            encoded_call,
        ]
        .concat();

        let len = Compact(
            u32::try_from(encoded_inner.len()).expect("extrinsic size expected to be <4GB"),
        )
        .encode();

        [len, encoded_inner].concat()
    };

    chain.submit(&extrinsic_call).await?;

    Ok(Response::Void)
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Response<'m> {
    Void,
    None,
    Value(scales::Value<'m>),
    ValueSet(Vec<(Vec<scales::Value<'m>>, Option<scales::Value<'m>>)>),
    Meta(&'m Metadata),
    Registry(&'m PortableRegistry),
}

impl From<Response<'_>> for Vec<u8> {
    fn from(res: Response) -> Self {
        match res {
            Response::Value(v) => v.as_ref().into(),
            Response::None => vec![0],
            Response::Meta(m) => m.encode(),
            Response::Registry(r) => r.encode(),
            Response::ValueSet(r) => r.encode(),
            Response::Void => vec![],
        }
    }
}

fn parse_uri(uri: &str) -> Option<(String, String, Vec<String>)> {
    let mut path = uri.trim_matches('/').split('/');
    let pallet = path.next().map(to_camel)?;
    let item = path.next().map(to_camel)?;
    let map_keys = path.map(to_camel).collect::<Vec<_>>();
    Some((pallet, item, map_keys))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StorageChangeSet {
    block: String,
    changes: Vec<(String, Option<String>)>,
}

pub type RawKey = Vec<u8>;
pub type RawValue = Vec<u8>;

/// Generic definition of a blockchain backend
///
/// ```rust,ignore
/// pub trait Backend {
///     async fn query_bytes(&self, key: &StorageKey) -> Result<Vec<u8>>;
///
///     async fn submit<T>(&self, ext: T) -> Result<()>
///     where
///         T: AsRef<[u8]>;
///
///     async fn metadata(&self) -> Result<Metadata>;
/// }
/// ```
pub trait Backend {
    async fn get_storage_items(
        &self,
        keys: Vec<RawKey>,
        block: Option<u32>,
    ) -> crate::Result<impl Iterator<Item = (RawKey, Option<RawValue>)>>;

    async fn get_storage_item(
        &self,
        key: RawKey,
        block: Option<u32>,
    ) -> crate::Result<Option<RawValue>> {
        let res = self.get_storage_items(vec![key], block).await?;
        log::info!("before it died");
        res.into_iter()
            .next()
            .map(|(_, v)| v)
            .ok_or(Error::StorageKeyNotFound)
    }

    async fn get_keys_paged(
        &self,
        from: RawKey,
        size: u16,
        to: Option<RawKey>,
    ) -> crate::Result<Vec<RawValue>>;

    /// Send a signed extrinsic to the blockchain
    async fn submit(&self, ext: impl AsRef<[u8]>) -> Result<()>;

    async fn metadata(&self) -> Result<Metadata>;

    async fn block_info(&self, at: Option<u32>) -> Result<meta::BlockInfo>;
}

/// A Dummy backend for offline querying of metadata
pub struct Offline(pub Metadata);

impl Backend for Offline {
    async fn get_storage_items(
        &self,
        _keys: Vec<RawKey>,
        _block: Option<u32>,
    ) -> crate::Result<impl Iterator<Item = (RawKey, Option<RawValue>)>> {
        Err::<Empty<(RawKey, Option<RawValue>)>, _>(Error::ChainUnavailable)
    }

    async fn get_keys_paged(
        &self,
        _from: RawKey,
        _size: u16,
        _to: Option<RawKey>,
    ) -> crate::Result<Vec<RawKey>> {
        Err(Error::ChainUnavailable)
    }

    /// Send a signed extrinsic to the blockchain
    async fn submit(&self, _ext: impl AsRef<[u8]>) -> Result<()> {
        Err(Error::ChainUnavailable)
    }

    async fn metadata(&self) -> Result<Metadata> {
        Ok(self.0.clone())
    }

    async fn block_info(&self, _: Option<u32>) -> Result<meta::BlockInfo> {
        Err(Error::ChainUnavailable)
    }
}

#[derive(Clone, Debug)]
pub enum Error {
    ChainUnavailable,
    BadInput,
    BadKey,
    BadMetadata,
    Decode(codec::Error),
    Encode(String),
    NoMetadataLoaded,
    Node(String),
    ParseStorageItem,
    StorageKeyNotFound,
    PalletNotFound(String),
    CallNotFound,
    MissingConstantName,
    Signing,
    Mapping(String),
    AccountNotFound,
    ConstantNotFound(String),
    Platform(String),
    CantInitBackend,
    CantDecodeReponseForMeta,
    CantDecodeRawQueryResponse,
    CantFindMethodInPallet,
    BadBlockNumber,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(e) => write!(f, "{:}", e),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(feature = "no_std")]
impl core::error::Error for Error {}
