#[cfg(any(feature = "http", feature = "http-web"))]
use crate::http::Backend as HttpBackend;
#[cfg(any(feature = "http", feature = "http-web", feature = "ws", feature = "js"))]
use crate::rpc::RpcClient;
#[cfg(feature = "ws")]
use crate::ws::Backend as WSBackend;
use crate::{
    meta::BlockInfo, Backend, Error, ExtrinsicBody, Metadata, Response, Result as SubeResult,
    Signer,
};
use crate::{prelude::*, Offline, RawKey, RawValue};

use core::future::{Future, IntoFuture};
use url::Url;

pub struct SubeBuilder<'a, Body, Signer> {
    url: Option<&'a str>,
    nonce: Option<u64>,
    body: Option<Body>,
    signer: Option<Signer>,
    metadata: Option<Metadata>,
}

impl<'a> Default for SubeBuilder<'a, (), ()> {
    fn default() -> Self {
        SubeBuilder {
            url: None,
            nonce: None,
            body: None,
            signer: None,
            metadata: None,
        }
    }
}

impl<'a> SubeBuilder<'a, (), ()> {
    pub fn with_url(self, url: &'a str) -> Self {
        Self {
            url: Some(url),
            ..self
        }
    }

    pub fn with_meta(self, metadata: Metadata) -> Self {
        Self {
            metadata: Some(metadata),
            ..self
        }
    }

    pub fn with_body<B>(self, body: B) -> SubeBuilder<'a, B, ()> {
        SubeBuilder {
            body: Some(body),
            url: self.url,
            nonce: self.nonce,
            signer: self.signer,
            metadata: self.metadata,
        }
    }

    async fn build_query(self) -> SubeResult<Response<'a>> {
        let Self { url, metadata, .. } = self;

        let url = chain_string_to_url(url.ok_or(Error::BadInput)?)?;

        let block = url
            .query_pairs()
            .find(|(k, _)| k == "at")
            .map(|(_, v)| v.parse::<u32>().expect("at query params must be a number"));

        let path = url.path();

        log::trace!("building the backend for {}", url);

        let (backend, meta) = get_multi_backend_by_url(url.clone(), metadata).await?;

        Ok(match path {
            "_meta" => Response::Meta(meta),
            "_meta/registry" => Response::Registry(&meta.types),
            _ => crate::query(&backend, meta, path, block).await?,
        })
    }
}

impl<'a, B> SubeBuilder<'a, B, ()> {
    pub fn with_signer<S>(self, signer: S) -> SubeBuilder<'a, B, S> {
        SubeBuilder {
            signer: Some(signer),
            body: self.body,
            metadata: self.metadata,
            nonce: self.nonce,
            url: self.url,
        }
    }
}

impl<'a, B, S> SubeBuilder<'a, B, S>
where
    B: serde::Serialize + core::fmt::Debug,
    S: Signer,
{
    pub fn with_nonce(self, nonce: u64) -> Self {
        Self {
            nonce: Some(nonce),
            ..self
        }
    }

    async fn build_extrinsic(self) -> SubeResult<Response<'a>> {
        let Self {
            url,
            nonce,
            body,
            signer,
            metadata,
            ..
        } = self;

        let url = chain_string_to_url(url.ok_or(Error::BadInput)?)?;
        let path = url.path();
        let body = body.ok_or(Error::BadInput)?;

        let (backend, meta) = get_multi_backend_by_url(url.clone(), metadata).await?;

        Ok(match path {
            "_meta" => Response::Meta(meta),
            "_meta/registry" => Response::Registry(&meta.types),
            _ => {
                let signer = signer.ok_or(Error::BadInput)?;

                crate::submit(backend, meta, path, ExtrinsicBody { nonce, body }, signer).await?
            }
        })
    }
}

use heapless::FnvIndexMap as Map;
use no_std_async::Mutex;

static INSTANCE_BACKEND: async_once_cell::OnceCell<
    Mutex<Map<String, Mutex<&'static AnyBackend>, 16>>,
> = async_once_cell::OnceCell::new();

static INSTANCE_METADATA: async_once_cell::OnceCell<
    Mutex<Map<String, Mutex<&'static Metadata>, 16>>,
> = async_once_cell::OnceCell::new();

async fn get_metadata(backend: &AnyBackend, metadata: Option<Metadata>) -> SubeResult<Metadata> {
    match metadata {
        Some(m) => Ok(m),
        None => backend.metadata().await.map_err(|_| Error::BadMetadata),
    }
}

async fn get_multi_backend_by_url<'a>(
    url: Url,
    metadata: Option<Metadata>,
) -> SubeResult<(&'a AnyBackend, &'a Metadata)> {
    let mut instance_backend = INSTANCE_BACKEND
        .get_or_init(async { Mutex::new(Map::new()) })
        .await
        .lock()
        .await;

    let mut instance_metadata = INSTANCE_METADATA
        .get_or_init(async { Mutex::new(Map::new()) })
        .await
        .lock()
        .await;

    let base_path = format!(
        "{}://{}:{}",
        url.scheme(),
        url.host_str().expect("url to have a host"),
        url.port().unwrap_or(80)
    );

    let cached_b = instance_backend.get(&base_path);
    let cached_m = instance_metadata.get(&base_path);

    match (cached_b, cached_m) {
        (Some(b), Some(m)) => {
            let b = *b.lock().await;
            let m = *m.lock().await;
            Ok((b, m))
        }
        _ => {
            let backend = Box::new(get_backend_by_url(url.clone()).await?);
            let backend = Box::leak::<'static>(backend);

            instance_backend
                .insert(base_path.clone(), Mutex::new(backend))
                .map_err(|_| Error::CantInitBackend)?;

            let metadata = Box::new(get_metadata(backend, metadata).await?);
            let metadata = Box::leak::<'static>(metadata);

            instance_metadata
                .insert(base_path.clone(), Mutex::new(metadata))
                .map_err(|_| Error::BadMetadata)?;

            Ok((backend, metadata))
        }
    }
}

pub type BoxFuture<'a, T> = core::pin::Pin<Box<dyn Future<Output = T> + 'a>>;

impl<'a> IntoFuture for SubeBuilder<'a, (), ()> {
    type Output = SubeResult<Response<'a>>;
    type IntoFuture = BoxFuture<'a, SubeResult<Response<'a>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.build_query())
    }
}

impl<'a, B, S> IntoFuture for SubeBuilder<'a, B, S>
where
    B: serde::Serialize + core::fmt::Debug + 'a,
    S: Signer + 'a,
{
    type Output = SubeResult<Response<'a>>;
    type IntoFuture = BoxFuture<'a, SubeResult<Response<'a>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.build_extrinsic())
    }
}

fn chain_string_to_url(chain: &str) -> SubeResult<Url> {
    let chain = if !chain.starts_with("ws://")
        && !chain.starts_with("wss://")
        && !chain.starts_with("http://")
        && !chain.starts_with("https://")
    {
        ["wss", chain].join("://")
    } else {
        chain.into()
    };

    let mut url = Url::parse(&chain).map_err(|_| Error::BadInput)?;

    if url.host_str().eq(&Some("localhost")) && url.port().is_none() {
        const WS_PORT: u16 = 9944;
        const HTTP_PORT: u16 = 9933;
        let port = match url.scheme() {
            "ws" => WS_PORT,
            _ => HTTP_PORT,
        };

        url.set_port(Some(port)).expect("known port");
    }

    Ok(url)
}

async fn get_backend_by_url(url: Url) -> SubeResult<AnyBackend> {
    match url.scheme() {
        #[cfg(feature = "ws")]
        "ws" | "wss" => Ok(AnyBackend::Ws(RpcClient(
            WSBackend::new_ws2(url.to_string().as_str()).await?,
        ))),
        #[cfg(any(feature = "http", feature = "http-web"))]
        "http" | "https" => Ok(AnyBackend::Http(RpcClient(HttpBackend::new(url)))),
        _ => Err(Error::BadInput),
    }
}

enum AnyBackend {
    #[cfg(any(feature = "http", feature = "http-web"))]
    Http(RpcClient<HttpBackend>),
    #[cfg(feature = "ws")]
    Ws(RpcClient<WSBackend>),
    _Offline(Offline),
}

impl Backend for &AnyBackend {
    async fn get_storage_items(
        &self,
        keys: Vec<RawKey>,
        block: Option<u32>,
    ) -> crate::Result<impl Iterator<Item = (RawKey, Option<RawValue>)>> {
        let result: Box<dyn Iterator<Item = (RawKey, Option<RawValue>)>> = match self {
            #[cfg(any(feature = "http", feature = "http-web"))]
            AnyBackend::Http(b) => Box::new(b.get_storage_items(keys, block).await?),
            #[cfg(feature = "ws")]
            AnyBackend::Ws(b) => Box::new(b.get_storage_items(keys, block).await?),
            AnyBackend::_Offline(b) => Box::new(b.get_storage_items(keys, block).await?),
        };

        Ok(result)
    }

    async fn get_storage_item(&self, key: RawKey, block: Option<u32>) -> crate::Result<Option<Vec<u8>>> {
        match self {
            #[cfg(any(feature = "http", feature = "http-web"))]
            AnyBackend::Http(b) => b.get_storage_item(key, block).await,
            #[cfg(feature = "ws")]
            AnyBackend::Ws(b) => b.get_storage_item(key, block).await,
            AnyBackend::_Offline(b) => b.get_storage_item(key, block).await,
        }
    }

    async fn get_keys_paged(
        &self,
        from: RawKey,
        size: u16,
        to: Option<RawKey>,
    ) -> crate::Result<Vec<RawKey>> {
        match self {
            #[cfg(any(feature = "http", feature = "http-web"))]
            AnyBackend::Http(b) => b.get_keys_paged(from, size, to).await,
            #[cfg(feature = "ws")]
            AnyBackend::Ws(b) => b.get_keys_paged(from, size, to).await,
            AnyBackend::_Offline(b) => b.get_keys_paged(from, size, to).await,
        }
    }

    async fn metadata(&self) -> SubeResult<Metadata> {
        match self {
            #[cfg(any(feature = "http", feature = "http-web"))]
            AnyBackend::Http(b) => b.metadata().await,
            #[cfg(feature = "ws")]
            AnyBackend::Ws(b) => b.metadata().await,
            AnyBackend::_Offline(b) => b.metadata().await,
        }
    }

    async fn submit(&self, ext: impl AsRef<[u8]>) -> SubeResult<()> {
        match self {
            #[cfg(any(feature = "http", feature = "http-web"))]
            AnyBackend::Http(b) => b.submit(ext).await,
            #[cfg(feature = "ws")]
            AnyBackend::Ws(b) => b.submit(ext).await,
            AnyBackend::_Offline(b) => b.submit(ext).await,
        }
    }

    async fn block_info(&self, at: Option<u32>) -> SubeResult<BlockInfo> {
        match self {
            #[cfg(any(feature = "http", feature = "http-web"))]
            AnyBackend::Http(b) => b.block_info(at).await,
            #[cfg(feature = "ws")]
            AnyBackend::Ws(b) => b.block_info(at).await,
            AnyBackend::_Offline(b) => b.block_info(at).await,
        }
    }
}

#[macro_export]
macro_rules! sube {

    ($url:expr) => {
        async {
            $crate::SubeBuilder::default().with_url($url).await
        }
    };

    ($url:expr => ($wallet:expr, $body:expr)) => {
        async {
            let mut builder = $crate::SubeBuilder::default();
            use $crate::Bytes;

            let public = $wallet.default_account().expect("to have a default account").public();

            let signer = $crate::SignerFn::from((public, |message: &[u8]| { 
                let message = message.to_vec();
                let wallet = &$wallet;
                async move {
                    let signature = wallet.sign(&message).await.map_err(|_| sube::Error::Signing)?;
                    Ok::<Bytes<64>, sube::Error>(signature.as_ref().try_into().unwrap())
                }
            }));

            builder
                .with_url($url)
                .with_body($body)
                .with_signer(signer)
                .await
                .map_err(|_| sube::Error::Signing)?;

            $crate::Result::Ok($crate::Response::Void)
        }
    };
}
