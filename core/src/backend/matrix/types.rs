use super::prelude::*;
use crate::base::StoreState;
use crate::utils::prelude::*;

#[derive(Serialize, Clone, Debug, Default, Deserialize, EventContent)]
#[ruma_event(type = "m.virto.apps", kind = GlobalAccountData)]
pub struct MatrixAppsStateContent {
    pub apps: StoreState,
}


#[async_trait]
pub trait MatrixSuperClient {
    fn client() -> Client;
    async fn wait_until_sync() -> ();
}


