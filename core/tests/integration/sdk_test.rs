pub mod client {

}

// #[cfg(test)]
// mod client_store_test {
//     use virto_sdk::prelude::*;
//     use virto_sdk::*;
//     use virto_sdk::std::wallet::{Wallet, WalletApi, WalletResult};
//     use virto_sdk::ConstructableService;

//     use tokio::test;

//     #[derive(Default)]
//     struct AuthConnectorMock;

//     #[async_trait::async_trait]
//     impl AuthenticatorBuilder for AuthConnectorMock {
//         async fn auth(
//             &self,
//             _: String,
//             client: MatrixClient,
//         ) -> Result<MatrixClient, AuthError> {
//             Ok(client)
//         }
//     }

//     #[tokio::test]
//     async fn login_with_credentials() {
//         let connector: AuthConnectorMock = AuthConnectorMock::default();

//         let service = SDKBuilder::new()
//             .with_homeserver("https://matrix-client.matrix.org")
//             .with_credentials("myfooaccoount", "H0l4mund0@123")
//             .build_and_login()
//             .await
//             .expect("error at building");
//     }

//     #[tokio::test]
//     async fn login_with_authenticator() {
//         let connector: AuthConnectorMock = AuthConnectorMock::default();
//         let service = SDKBuilder::new()
//             .with_homeserver("https://matrix-client.matrix.org")
//             .with_authenticator(Box::new(AuthConnectorMock::default()))
//             .build_and_login()
//             .await
//             .expect("error at building");
//     }

//     type WalletAggregate = Wallet;

//     #[tokio::test]
//     async fn craft_app() {
//         let service = WalletAggregate::default();

//         let mut sdk = SDKBuilder::new()
//             .with_homeserver("https://matrix-client.matrix.org")
//             .with_credentials("myfooaccoount", "H0l4mund0@123")
//             .build_and_login()
//             .await
//             .expect("error at building");
//     }

//     #[derive(Default)]
//     struct MockWalletService;

//     impl ConstructableService for MockWalletService {
//         type Args = ();
//         type Service = MockWalletService;

//         fn new(args: Self::Args) -> Self::Service {
//             MockWalletService
//         }
//     }

//     #[async_trait]
//     impl WalletApi for MockWalletService {
//         async fn sign<'p>(&self, payload: &'p [u8]) -> WalletResult<Message> {
//             Ok(Message::try_from([1u8].as_slice()).expect("hello world"))
//         }
//     }
// }
