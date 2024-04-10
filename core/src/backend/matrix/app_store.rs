use crate::base::{
    AppInfo, AppMetadata, AppRegistryError, Store, StoreResult, StoreState,
};

use super::prelude::*;
use crate::utils::prelude::*;

#[derive(Debug, Clone)]
pub struct MatrixAppStore {
    client: Client,
}

impl MatrixAppStore {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn get_state(&self) -> Result<MatrixAppsStateContent, AppRegistryError> {
        let account_data_vapp = self
            .client
            .account()
            .account_data::<MatrixAppsStateContent>()
            .await
            .map_err(|_| AppRegistryError::Unknown)?;

        let raw_vapp = account_data_vapp.unwrap_or(
            Raw::new(&MatrixAppsStateContent::default()).map_err(|_| AppRegistryError::Unknown)?,
        );

        Ok(raw_vapp
            .deserialize()
            .map_err(|_| AppRegistryError::Unknown)?)
    }

    async fn add_app(
        &self,
        app_info: &AppInfo,
        room: Room,
    ) -> Result<StoreState, AppRegistryError> {
        let mut apps = self.get_state().await?;

        apps.apps.insert(
            app_info.id.clone().into(),
            AppMetadata {
                app_info: app_info.clone(),
                channel_id: room.room_id().to_string(),
            },
        );

        self.client
            .account()
            .set_account_data(apps.clone())
            .await
            .map_err(|_| AppRegistryError::Unknown)?;

        Ok(apps.apps.clone())
    }

    async fn remove_app(&self, app_info: &AppInfo) -> Result<StoreState, AppRegistryError> {
        let mut vapps = self.get_state().await?;
        vapps.apps.remove(&app_info.id);

        self.client
            .account()
            .set_account_data(vapps.clone())
            .await
            .map_err(|_| AppRegistryError::Unknown)?;

        Ok(vapps.apps.clone())
    }

    fn get_room_id(&self, app_info: &AppInfo) -> String {
        format!("app-{}", app_info.id)
    }

    fn get_room(&self, app_info: &AppInfo) -> Option<Room> {
        self.client
            .joined_rooms()
            .iter()
            .find(|r| r.name().eq(&Some(self.get_room_id(app_info))))
            .map(|r| r.to_owned())
    }
}


impl Store for MatrixAppStore {
    async fn add(&self, info: &AppInfo) -> StoreResult<StoreState> {
        if self.is_registered(&info.id).await? {
            return Err(AppRegistryError::AlreadyInstalled);
        }
        let mut room = CreateRoomRequest::new();

        room.visibility = Visibility::Private;
        room.name = Some(self.get_room_id(info));
        room.initial_state =
            vec![
                InitialStateEvent::new(RoomEncryptionEventContent::with_recommended_defaults())
                    .to_raw_any(),
            ];

        let room = self
            .client
            .create_room(room)
            .await
            .map_err(|e| AppRegistryError::CantAddApp(e.to_string()))?;

        self.add_app(info, room).await
    }

    async fn remove(&self, info: &AppInfo) -> StoreResult<StoreState> {
        let room = self.get_room(info).ok_or(AppRegistryError::CantUninstall(
            "Can't get installed room".to_string(),
        ))?;

        room.leave()
            .await
            .map_err(|_| AppRegistryError::CantUninstall("Can't leave the room".to_string()))?;

        room.forget()
            .await
            .map_err(|_| AppRegistryError::CantUninstall("Can't forget the room".to_string()))?;

        self.remove_app(info).await
    }

    async fn is_registered(&self, id: &str) -> StoreResult<bool> {
        let state = self.get_state().await?;

        Ok(state.apps.get(id).is_some())
    }

    async fn list_apps(&self) -> StoreResult<Vec<AppInfo>> {
        let state = self.get_state().await?;
        Ok(state
            .apps
            .values()
            .into_iter()
            .map(|x| x.app_info.to_owned())
            .collect())
    }
}
