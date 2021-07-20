use async_trait::async_trait;
pub use codec;
use codec::Decode;
use core::future::Future;
use frame_metadata::v12::{StorageEntryType, StorageHasher};
pub use frame_metadata::RuntimeMetadataPrefixed;
use futures_lite::AsyncRead;
use hasher::hash;
use meta_ext::MetaExt;
use once_cell::sync::OnceCell;
use std::{
    convert::{TryFrom, TryInto},
    fmt,
    ops::Deref,
    str::FromStr,
};

#[cfg(feature = "http")]
pub mod http;
// #[cfg(feature = "ws")]
pub mod ws;

mod hasher;
mod meta_ext;
mod rpc;

static META_REF: OnceCell<RuntimeMetadataPrefixed> = OnceCell::new();

/// Submit extrinsics
#[derive(Debug)]
pub struct Sube<T>(T);

impl<T: Backend> Sube<T> {
    /// Get or set if not available the chain metadata that all instances of Sube
    /// will share.
    /// Metadata will be held as a static global to allow for convenient conversion of
    /// types like string literals to a metadata aware `StorageKey` without the user having to
    /// provide their own metadata object to a less ergonomic conversion method.
    pub async fn get_or_try_init_meta<F, M>(f: F) -> Result<&'static RuntimeMetadataPrefixed>
    where
        F: FnOnce() -> M,
        M: Future<Output = Result<RuntimeMetadataPrefixed>>,
    {
        if let Some(meta) = META_REF.get() {
            return Ok(meta);
        };
        let meta = f().await?;
        Ok(META_REF.get_or_init(|| meta))
    }

    pub async fn try_init_meta(&self) -> Result<&'static RuntimeMetadataPrefixed> {
        Self::get_or_try_init_meta(|| self.0.metadata()).await
    }
}

impl<T: Backend> From<T> for Sube<T> {
    fn from(b: T) -> Self {
        Sube(b)
    }
}

impl<T: Backend> Deref for Sube<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Generic backend definition
#[async_trait]
pub trait Backend {
    /// Get storage items form the blockchain
    async fn query_bytes<K>(&self, key: K) -> Result<Vec<u8>>
    where
        K: TryInto<StorageKey, Error = Error> + Send;

    async fn query<K, R>(&self, key: K) -> Result<R>
    where
        K: TryInto<StorageKey, Error = Error> + Send,
        R: codec::Decode,
    {
        let res = self.query_bytes(key).await?;
        Decode::decode(&mut res.as_ref()).map_err(|e| Error::Decode(e))
    }

    /// Send a signed extrinsic to the blockchain
    async fn submit<T>(&self, ext: T) -> Result<()>
    where
        T: AsyncRead + Send + Unpin;

    async fn metadata(&self) -> Result<RuntimeMetadataPrefixed>;
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    ChainUnavailable,
    BadInput,
    BadKey,
    BadMetadata,
    Decode(codec::Error),
    NoMetadataLoaded,
    Node(String),
    ParseStorageItem,
    StorageKeyNotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(e) => write!(f, "{:}", e),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl From<async_tungstenite::tungstenite::Error> for Error {
    fn from(_err: async_tungstenite::tungstenite::Error) -> Self {
        Error::ChainUnavailable
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct StorageKey(Vec<u8>);

impl StorageKey {
    fn get_global_metadata() -> Result<&'static RuntimeMetadataPrefixed> {
        META_REF.get().ok_or(Error::NoMetadataLoaded)
    }

    fn from_parts(module: &str, item: &str, k1: Option<&str>, k2: Option<&str>) -> Result<Self> {
        log::debug!(
            "StorageKey parts: [module]={} [item]={} [key1]={} [key2]={}",
            module,
            item,
            k1.unwrap_or("()"),
            k2.unwrap_or("()"),
        );
        let meta = Self::get_global_metadata()?;
        let entry = meta.entry(module, item).ok_or(Error::StorageKeyNotFound)?;

        let mut key = hash(&StorageHasher::Twox128, &module);
        key.append(&mut hash(&StorageHasher::Twox128, &item));

        let key = match entry.ty {
            StorageEntryType::Plain(_) => key,
            StorageEntryType::Map { ref hasher, .. } => {
                if k1.is_none() || k1.as_ref().unwrap().is_empty() {
                    return Err(Error::StorageKeyNotFound);
                }
                key.append(&mut hash(hasher, &k1.unwrap()));
                key
            }
            StorageEntryType::DoubleMap {
                ref hasher,
                ref key2_hasher,
                ..
            } => {
                if (k1.is_none() || k1.as_ref().unwrap().is_empty())
                    || (k2.is_none() || k2.as_ref().unwrap().is_empty())
                {
                    return Err(Error::StorageKeyNotFound);
                }
                key.append(&mut hash(hasher, &k1.unwrap()));
                key.append(&mut hash(key2_hasher, &k2.unwrap()));
                key
            }
        };

        Ok(StorageKey(key))
    }
}

impl FromStr for StorageKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        // asume it's a path like "module/item-name"
        let mut path = s.split('/');
        let module = path.next().map(to_camel).ok_or(Error::ParseStorageItem)?;
        let item = path.next().map(to_camel).ok_or(Error::ParseStorageItem)?;
        let k1 = path.next();
        let k2 = path.next();
        StorageKey::from_parts(&module, &item, k1, k2)
    }
}

impl TryFrom<&str> for StorageKey {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self> {
        s.parse()
    }
}

impl<T: AsRef<str>> From<(T, T)> for StorageKey {
    fn from((m, it): (T, T)) -> Self {
        StorageKey::from_parts(m.as_ref(), it.as_ref(), None, None)
            .expect("valid module and item names")
    }
}

impl fmt::Display for StorageKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl AsRef<[u8]> for StorageKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

fn to_camel(term: &str) -> String {
    let underscore_count = term.chars().filter(|c| *c == '-').count();
    let mut result = String::with_capacity(term.len() - underscore_count);
    let mut at_new_word = true;

    for c in term.chars().skip_while(|&c| c == '-') {
        if c == '-' {
            at_new_word = true;
        } else if at_new_word {
            result.push(c.to_ascii_uppercase());
            at_new_word = false;
        } else {
            result.push(c);
        }
    }
    result
}
