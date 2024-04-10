
#[cfg(test)]
mod manager_test {
    use ::std::error::Error;

    use super::MatrixAppStore;
    use crate::{base::AppInfo, base::Store, SDKBuilder, SDKCore};
    use async_once_cell::OnceCell;
    use ctor::ctor;
    use tokio::test;
    use tokio::time::{sleep, Duration};
    use tracing_subscriber::fmt::init as InitLogger;

    static mut SDK_CORE: OnceCell<SDKCore> = OnceCell::new();

    async fn get_sdk() -> &'static mut SDKCore {
        unsafe {
            match SDK_CORE.get() {
                Some(..) => SDK_CORE.get_mut().expect("error getting mut"),
                None => {
                    SDK_CORE
                        .get_or_try_init(async {
                            let mut core = SDKBuilder::new()
                                .with_homeserver("https://matrix-client.matrix.org")
                                .with_credentials("fooaccount2", "H0l4mund0@123")
                                .with_device_name("iphone-dev")
                                .build_and_login()
                                .await
                                .expect("Can not login");

                            core.init().await;

                            Ok::<_, ()>(core)
                        })
                        .await
                        .expect("Erroor at getting core");

                    SDK_CORE.get_mut().expect("error getting mut")
                }
            }
        }
    }

    #[ctor]
    fn before() {
        // InitLogger();
    }

    #[tokio::test]
    async fn app_registry_lifecycle() {
        let sdkCore = get_sdk().await;

        let app_info = AppInfo {
            description: "foo".into(),
            name: "wallet".into(),
            id: "com.virto.wallet".into(),
            author: "hello@virto.net".into(),
            version: "0.0.1".into(),
            permissions: vec![],
        };

        sdkCore.next_sync().await;

        let manager = MatrixAppStore::new(sdkCore.client());

        manager.remove_app(&app_info).await;
        let state_d = manager.get_state().await.expect("hello world");

        assert_eq!(
            manager
                .is_registered(&app_info.id)
                .await
                .expect("error checking installed app"),
            false
        );

        assert!(manager.add(&app_info).await.is_ok());

        sdkCore.next_sync().await;

        let state = manager.get_state().await.expect("hello");

        assert_eq!(state.apps.get(&app_info.id).unwrap().app_info, app_info);
        
        let state = manager.list_apps().await.expect("It must list the apps");
        
        assert_eq!(state.len(), 1);
        
        sdkCore.next_sync().await;
        
        assert!(manager.remove(&app_info).await.is_ok());

        sdkCore.next_sync().await;

        let state = manager.get_state().await.expect("cant get reload event");

        assert!(state.apps.get(&app_info.id).is_none());
    }
}
