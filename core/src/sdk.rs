use crate::{backend::matrix::MatrixAppStore, utils::prelude::*};

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingHomeServer,
    Cancelled,
    Unknown,
}

#[async_trait]
pub trait AuthenticatorBuilder {
    async fn auth(
        &self,
        device_name: String,
        client: MatrixClient,
    ) -> Result<MatrixClient, AuthError>;
}

pub enum SDKError {
    Unknown,
    CantSync,
    AlreadyInit,
}

pub struct SDKCore {
    inner: MatrixClient,
    is_init: bool,
    next_batch_token: Option<String>,
    manager: MatrixAppStore,
    // supervisor: SimpleSuperVisor<'app>,
    // apps: Vec<&'app dyn VRunnableApp>,
}

impl SDKCore {
    pub(crate) fn new(inner: MatrixClient) -> Self {
        return Self {
            inner: inner.clone(),
            is_init: false,
            next_batch_token: None,
            manager: MatrixAppStore::new(inner.clone()),
        };
    }

    pub fn client(&self) -> MatrixClient {
        if !self.is_init {
            panic!("accessing client before its initialization");
        }
        self.inner.clone()
    }

    pub async fn next_sync(&mut self) -> Result<(), SDKError> {
        let mut settings = MatrixSyncSettings::default();

        if let Some(token) = &self.next_batch_token {
            settings = settings.token(token)
        }

        let sync_res = self
            .inner
            .sync_once(settings)
            .await
            .map_err(|_| SDKError::CantSync)?;

        self.next_batch_token = Some(sync_res.next_batch);
        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), SDKError> {
        if self.is_init {
            return Err(SDKError::AlreadyInit);
        }
        self.is_init = true;
        self.next_sync();
        Ok(())
    }

    /**
     * Blocking Call
     */
    pub async fn start(&self) -> Result<(), SDKError> {
        let mut settings = MatrixSyncSettings::default();

        if let Some(token) = &self.next_batch_token {
            settings = settings.token(token)
        }

        self.inner
            .sync(settings)
            .await
            .map_err(|_| SDKError::CantSync)?;
        Ok(())
    }
}

pub struct SDKBuilder {
    homeserver_url: Option<String>,
    credentials: Option<(String, String)>,
    custom_authenticator: Option<Box<dyn AuthenticatorBuilder + Send>>,
    client: Option<Box<MatrixClient>>,
    device_name: Option<String>,
}

impl SDKBuilder {
    pub fn new() -> Self {
        Self {
            homeserver_url: None,
            client: None,
            custom_authenticator: None,
            credentials: None,
            device_name: None,
        }
    }

    pub fn with_homeserver(self, homeserver_url: impl Into<String>) -> Self {
        Self {
            homeserver_url: Some(homeserver_url.into()),
            ..self
        }
    }

    pub fn with_device_name(self, id: impl Into<String>) -> Self {
        Self {
            device_name: Some(id.into()),
            ..self
        }
    }

    pub fn with_authenticator(
        self,
        custom_authenticator: Box<dyn AuthenticatorBuilder + Sync + Send>,
    ) -> Self {
        Self {
            custom_authenticator: Some(custom_authenticator),
            ..self
        }
    }

    pub fn with_credentials(
        self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            credentials: Some((username.into(), password.into())),
            ..self
        }
    }

    pub async fn build_and_login(self) -> Result<SDKCore, AuthError> {
        let Self {
            custom_authenticator,
            credentials,
            homeserver_url,
            ..
        } = self;

        let homeserver_url = homeserver_url.ok_or(AuthError::MissingHomeServer)?;
        let mut client = MatrixClient::new(Url::parse(&homeserver_url).expect("Wrong Url"))
            .await
            .map_err(|_| AuthError::Unknown)?;

        if let Some(authenticator) = custom_authenticator {
            client = authenticator
                .auth(
                    self.device_name.unwrap_or("VitoSDK".to_string()),
                    client.clone(),
                )
                .await?;
        } else {
            let (username, password) = credentials.ok_or(AuthError::WrongCredentials)?;
            client
                .matrix_auth()
                .login_username(&username, &password)
                .initial_device_display_name(&self.device_name.unwrap_or("VitoSDK".to_string()))
                .send()
                .await
                .map_err(|_| AuthError::WrongCredentials)?;
        }

        Ok(SDKCore::new(client))
    }
}
