use crate::prelude::*;
use core::{borrow::Borrow, slice};

use codec::Decode;
#[cfg(any(feature = "v13"))]
use frame_metadata::decode_different::DecodeDifferent;

#[cfg(any(feature = "v14"))]
use frame_metadata::v14::PalletCallMetadata;
use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed};

#[cfg(feature = "v13")]
pub use v13::*;
#[cfg(feature = "v14")]
pub use v14::*;

use crate::hasher::hash;

#[cfg(feature = "v13")]
mod v13 {
    use frame_metadata::v13::*;
    pub type Metadata = RuntimeMetadataV13;
    pub type ExtrinsicMeta = ExtrinsicMetadata;
    pub type PalletMeta = ModuleMetadata;
    pub type StorageMeta = StorageMetadata;
    pub type EntryMeta = StorageEntryMetadata;
    pub type EntryType = StorageEntryType;
    pub type Hasher = StorageHasher;
}
#[cfg(feature = "v14")]
mod v14 {
    use frame_metadata::v14::*;
    use scale_info::form::PortableForm;
    pub type Metadata = RuntimeMetadataV14;
    pub type ExtrinsicMeta = ExtrinsicMetadata;
    pub type PalletMeta = PalletMetadata<PortableForm>;
    pub type CallsMeta = PalletCallMetadata<PortableForm>;
    pub type StorageMeta = PalletStorageMetadata<PortableForm>;
    pub type ConstMeta = PalletConstantMetadata<PortableForm>;
    pub type EntryMeta = StorageEntryMetadata<PortableForm>;
    pub type EntryType = StorageEntryType<PortableForm>;
    pub type Hasher = StorageHasher;
    pub use scale_info::PortableRegistry;
    pub type Type = scale_info::Type<PortableForm>;
    pub type TypeDef = scale_info::TypeDef<PortableForm>;
}

/// Decode metadata from its raw prefixed format to the currently
/// active metadata version.
pub fn from_bytes(bytes: &mut &[u8]) -> core::result::Result<Metadata, codec::Error> {
    let meta: RuntimeMetadataPrefixed = Decode::decode(bytes)?;
    
    let meta = match meta.1 {
        #[cfg(feature = "v13")]
        RuntimeMetadata::V13(m) => m,
        #[cfg(feature = "v14")]
        RuntimeMetadata::V14(m) => m,
        _ => unreachable!("Metadata version not supported"),
    };
    Ok(meta)
}

pub struct BlockInfo {
    pub number: u64,
    pub hash: [u8; 32],
    pub parent: [u8; 32],
}

impl Into<Vec<u8>> for BlockInfo {
    fn into(self) -> Vec<u8> {
        self.hash.into()
    }
}

type Entries<'a, E> = slice::Iter<'a, E>;

/// An extension trait for a decoded metadata object that provides
/// convenient methods to navigate and extract data from it.
pub trait Meta<'a> {
    type Pallet: Pallet<'a>;

    fn pallets(&self) -> Pallets<Self::Pallet>;

    fn pallet_by_name(&self, name: &str) -> Option<&Self::Pallet> {
        self.pallets()
            .find(|p| p.name().to_lowercase() == name.to_lowercase())
    }
    fn cloned(&self) -> Self;
}

#[cfg(feature = "v14")]
pub trait Registry {
    fn find_ids(&self, q: &str) -> Vec<u32>;
    fn find(&self, q: &str) -> Vec<&scale_info::Type<scale_info::form::PortableForm>>;
}

type Pallets<'a, P> = slice::Iter<'a, P>;

pub trait Pallet<'a> {
    type Storage: Storage<'a>;

    fn name(&self) -> &str;
    fn storage(&self) -> Option<&Self::Storage>;
    fn calls(&self) -> Option<&CallsMeta>;
}

pub trait Storage<'a> {
    type Entry: Entry + 'a;

    fn module(&self) -> &str;
    fn entries(&'a self) -> Entries<'a, Self::Entry>;

    fn entry(&'a self, name: &str) -> Option<&'a Self::Entry> {
        self.entries().find(|e| e.name() == name)
    }
}

pub trait Entry {
    type Type: EntryTy;

    fn name(&self) -> &str;
    fn ty(&self) -> &Self::Type;
    fn ty_id(&self) -> u32;

    fn key<T: AsRef<str>>(&self, pallet: &str, map_keys: &[T]) -> Option<Vec<u8>> {
        self.ty().key(pallet, self.name(), map_keys)
    }
}

pub trait EntryTy {
    fn key<T: AsRef<str>>(&self, pallet: &str, item: &str, map_keys: &[T]) -> Option<Vec<u8>>;

    fn build_key<H, T>(
        &self,
        pallet: &str,
        item: &str,
        map_keys: &[T],
        hashers: &[H],
    ) -> Option<Vec<u8>>
    where
        H: Borrow<Hasher>,
        T: AsRef<str>,
    {
        let mut key = hash(&Hasher::Twox128, &pallet);
        key.append(&mut hash(&Hasher::Twox128, &item));

        if map_keys.len() != hashers.len() {
            return None;
        }

        for (i, h) in hashers.iter().enumerate() {
            let hasher = h.borrow();
            let k = map_keys[i].as_ref();
            if k.is_empty() {
                return None;
            }
            key.append(&mut hash(hasher, k))
        }
        Some(key)
    }
}

impl<'a> Meta<'a> for Metadata {
    type Pallet = PalletMeta;
    #[cfg(not(feature = "v14"))]
    fn pallets(&self) -> Pallets<Self::Pallet> {
        self.modules.decoded().iter()
    }
    #[cfg(feature = "v14")]
    fn pallets(&self) -> Pallets<Self::Pallet> {
        self.pallets.iter()
    }
    fn cloned(&self) -> Self {
        #[cfg(feature = "v14")]
        let meta = self.clone();
        #[cfg(not(feature = "v14"))]
        let meta = {
            Metadata {
                modules: self.modules.clone(),
                extrinsic: ExtrinsicMeta {
                    version: self.extrinsic.version,
                    signed_extensions: self.extrinsic.signed_extensions.clone(),
                },
            }
        };
        meta
    }
}

#[cfg(feature = "v14")]
impl Registry for scale_info::PortableRegistry {
    fn find_ids(&self, path: &str) -> Vec<u32> {
        self.types()
            .iter()
            .filter(|t| {
                t.ty()
                    .path()
                    .segments()
                    .iter()
                    .any(|s| s.to_lowercase().contains(&path.to_lowercase()))
            })
            .map(|t| t.id())
            .collect()
    }
    fn find(&self, path: &str) -> Vec<&scale_info::Type<scale_info::form::PortableForm>> {
        self.find_ids(path)
            .into_iter()
            .map(|t| self.resolve(t).unwrap())
            .collect()
    }
}

impl<'a> Pallet<'a> for PalletMeta {
    type Storage = StorageMeta;
    fn storage(&self) -> Option<&Self::Storage> {
        let storage = self.storage.as_ref();
        #[cfg(not(feature = "v14"))]
        let storage = storage.map(|s| s.decoded());
        storage
    }

    fn name(&self) -> &str {
        #[cfg(feature = "v14")]
        let name = self.name.as_ref();
        #[cfg(not(feature = "v14"))]
        let name = self.name.decoded();
        name
    }

    fn calls(&self) -> Option<&CallsMeta> {
        self.calls.as_ref()
    }
}

impl<'a> Storage<'a> for StorageMeta {
    type Entry = EntryMeta;

    fn module(&self) -> &str {
        #[cfg(feature = "v14")]
        let pref = self.prefix.as_ref();
        #[cfg(not(feature = "v14"))]
        let pref = self.prefix.decoded();
        pref
    }

    fn entries(&'a self) -> Entries<Self::Entry> {
        #[cfg(feature = "v14")]
        let entries = self.entries.iter();
        #[cfg(not(feature = "v14"))]
        let entries = self.entries.decoded().iter();
        entries
    }
}

impl<'a> Entry for EntryMeta {
    type Type = EntryType;

    fn name(&self) -> &str {
        #[cfg(feature = "v14")]
        let name = self.name.as_ref();
        #[cfg(not(feature = "v14"))]
        let name = self.name.decoded();
        name
    }
    fn ty(&self) -> &Self::Type {
        &self.ty
    }

    fn ty_id(&self) -> u32 {
        #[cfg(feature = "v14")]
        match &self.ty {
            EntryType::Plain(t) => t.id(),
            EntryType::Map { value, .. } => value.id(),
        }
        #[cfg(not(feature = "v14"))]
        0
    }
}

impl EntryTy for EntryType {
    fn key<T: AsRef<str>>(&self, pallet: &str, item: &str, map_keys: &[T]) -> Option<Vec<u8>> {
        match self {
            Self::Plain(ty) => {
                log::debug!("Item Type: {:?}", ty);
                self.build_key::<Hasher, &str>(pallet, item, &[], &[])
            }
            #[cfg(any(feature = "v13"))]
            Self::Map {
                hasher, key, value, ..
            } => {
                log::debug!("Item Type: {:?} => {:?}", key, value);
                self.build_key(pallet, item, map_keys, &[hasher])
            }
            #[cfg(any(feature = "v13"))]
            Self::DoubleMap {
                hasher,
                key2_hasher,
                key1,
                key2,
                value,
            } => {
                log::debug!("Item Type: ({:?}, {:?}) => {:?}", key1, key2, value);
                self.build_key(pallet, item, map_keys, &[hasher, key2_hasher])
            }
            #[cfg(feature = "v13")]
            Self::NMap {
                hashers,
                keys,
                value,
            } => {
                log::debug!("Item Type: {:?} => {:?}", keys, value);
                self.build_key(pallet, item, map_keys, hashers.decoded())
            }
            #[cfg(feature = "v14")]
            Self::Map {
                hashers,
                key,
                value,
            } => {
                log::debug!("Item Type: {:?} => {:?}", key, value);
                self.build_key(pallet, item, map_keys, hashers)
            }
        }
    }
}

#[cfg(any(feature = "v13"))]
trait Decoded {
    type Output;
    fn decoded(&self) -> &Self::Output;
}

#[cfg(any(feature = "v13"))]
impl<B, O> Decoded for DecodeDifferent<B, O> {
    type Output = O;
    fn decoded(&self) -> &Self::Output {
        match self {
            DecodeDifferent::Decoded(o) => o,
            _ => unreachable!(),
        }
    }
}
