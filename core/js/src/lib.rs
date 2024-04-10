use virto_sdk::run_me;

// mod models;
// pub mod utils;
// use models::*;

// mod prelude {
//     pub use crate::utils::{self, AsJsError};
//     pub use async_trait::async_trait;
//     pub use base64;
//     pub use core::fmt::Debug;
//     pub use js_sys::Uint8Array;
//     pub use serde::{
//         de::{Error as DeError, SeqAccess, Visitor},
//         Deserialize, Deserializer, Serialize, Serializer,
//     };
//     pub use virto_sdk::{
//         authenticator::AuthError,
//         signer::SignerError,
//         // messages::{Chat, ChatResult, Message},
//         transport::Transport,
//         AuthResult,
//         Authenticator,
//         Signer,
//         SignerResult,
//         VirtoSDK,
//     };
//     pub use wasm_bindgen::prelude::*;
//     pub use wasm_bindgen_futures::JsFuture;
//     pub use web_sys::{
//         window, CredentialCreationOptions, CredentialRequestOptions,
//         PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
//         PublicKeyCredentialRpEntity, PublicKeyCredentialUserEntity,
//     };
// }

// use prelude::*;
// use virto_sdk::transport::{Content, TransportResult};

// pub struct Profile<'a> {
//     username: &'a str,
//     display_name: &'a str,
// }
// pub struct Credentials<'a> {
//     username: &'a str,
// }

// pub struct WebAuthN {
//     rp_id: String,
// }

// #[async_trait(?Send)]
// impl Authenticator for WebAuthN {
//     type Credentials<'c> = Credentials<'c>;
//     type Profile<'p> = Profile<'p>;
//     type RegResponse = WebAuthnResult<RegResponse>;

//     async fn register<'m>(&self, profile: &'m Profile<'_>) -> AuthResult<Self::RegResponse> {
//         let window = window().ok_or(AuthError::Platform("No window defined".into()))?;
//         let credential_manager = window.navigator().credentials();
//         let mut credential_options = CredentialCreationOptions::new();

//         let mut slice: [u8; 32] = [0; 32];

//         let crypto = window
//             .crypto()
//             .map_err(|_| AuthError::Platform("crypto is not defined".into()))?;
//         crypto
//             .get_random_values_with_u8_array(&mut slice)
//             .map_err(|_| AuthError::Platform("Can't be generated a random challenge".into()))?;

//         let challenge = Uint8Array::from(slice.as_slice());
//         let id = &challenge;

//         let creds = vec![
//             KeyType {
//                 alg: utils::signing_algorithm::RSA,
//                 type_credential: "public-key".to_string(),
//             },
//             KeyType {
//                 alg: utils::signing_algorithm::EDSA,
//                 type_credential: "public-key".to_string(),
//             }, // EDSA
//         ];

//         let pub_key = PublicKeyCredentialCreationOptions::new(
//             &challenge,
//             &serde_wasm_bindgen::to_value(&creds).unwrap(),
//             &PublicKeyCredentialRpEntity::new(&self.rp_id),
//             &PublicKeyCredentialUserEntity::new(profile.username, profile.display_name, id),
//         );

//         credential_options.public_key(&pub_key);

//         let promise = credential_manager
//             .create_with_options(&credential_options)
//             .map_err(|_| AuthError::CanNotRegister)?;

//         let value = JsFuture::from(promise)
//             .await
//             .map_err(|_| AuthError::CanNotRegister)?;

//         let public_key_cred = serde_wasm_bindgen::from_value(value)
//             .map_err(|_| AuthError::Platform("Error Serializing Response".into()))?;

//         Ok(public_key_cred)
//     }

//     async fn auth<'m>(&self, _: &'m Credentials<'_>) -> AuthResult<()> {
//         Ok(())
//     }
// }

// #[async_trait(?Send)]
// impl Signer for WebAuthN {
//     type SignedPayload = WebAuthnResult<SigningResponse>;

//     async fn sign<'p>(&self, payload: &'p [u8]) -> SignerResult<<Self as Signer>::SignedPayload> {
//         let window = window().ok_or(SignerError::Platform("No window defined".into()))?;
//         let credential_manager = window.navigator().credentials();
//         let mut credential_request = CredentialRequestOptions::new();
//         let challenge = Uint8Array::from(payload);

//         let mut pub_req = PublicKeyCredentialRequestOptions::new(&challenge);

//         pub_req.rp_id(&self.rp_id);

//         js_sys::Reflect::set(
//             pub_req.as_ref(),
//             &JsValue::from("attestation"),
//             &JsValue::from("indirect"),
//         )
//         .map_err(|_| SignerError::Platform("Error Serializing Response".into()))?;

//         credential_request.public_key(&pub_req);

//         let promise = credential_manager
//             .get_with_options(&credential_request)
//             .map_err(|_| SignerError::WrongCredentials)?;
//         let value = JsFuture::from(promise)
//             .await
//             .map_err(|_| SignerError::WrongCredentials)?;
//         let public_key_cred = serde_wasm_bindgen::from_value(value).map_err(|_| {
//             SignerError::Platform("Error serializing signature from webauthn".into())
//         })?;

//         Ok(public_key_cred)
//     }
// }

// impl WebAuthN {
//     pub fn new(rp_id: String) -> WebAuthN {
//         WebAuthN { rp_id }
//     }
// }

// struct ChatJs;

// #[async_trait]
// impl Transport for ChatJs {
//     async fn send<'s, Body: Serialize + Debug + Send>(
//         key: &'s str,
//         content: Content<Body>,
//     ) -> TransportResult<()> {
//         Ok(())
//     }
// }

// type SDK = VirtoSDK<WebAuthN, WebAuthN, ChatJs>;

// #[wasm_bindgen]
// struct VirtoSDKJs {
//     sdk: SDK,
// }

// // #[wasm_bindgen]
// // impl VirtoSDKJs {

// // }
