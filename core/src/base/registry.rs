use crate::AppLoader;
use crate::AppInfo;
use crate::utils::prelude::*;

#[derive(Debug)]
pub enum AppRegistryError {
    AlreadyInstalled,
    Unknown,
    CantAddApp(String),
    CantUninstall(String),
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppMetadata {
    pub app_info: AppInfo,
    pub channel_id: String,
}

pub type StoreResult<T> = Result<T, AppRegistryError>;
pub type StoreState = HashMap<String, AppMetadata>;

pub trait Store {
    async fn is_registered(&self, id: &str) -> StoreResult<bool>;
    async fn add(&self, app_info: &AppInfo) -> StoreResult<StoreState>;
    async fn remove(&self, id: &AppInfo) -> StoreResult<StoreState>;
    async fn list_apps(&self) -> StoreResult<Vec<AppInfo>>;
}


pub type RegistryResult<T> = Result<T, AppRegistryError>;

pub enum AppRegisteredLoader {
    NotInstalled
}

#[async_trait]
pub trait AppRegistry: Sync + Send  {
    async fn install<'r>(&self, loader: &Box<dyn AppLoader<'r>>) -> RegistryResult<()>;
    async fn get_loader<'r>(&self, id: &str) -> Option<&Box<dyn AppLoader<'r>>>;
}

