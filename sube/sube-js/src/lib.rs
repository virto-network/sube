mod util;

use core::convert::TryInto;
use js_sys::Uint8Array;
use log::Level;
use parity_scale_codec::Encode;
use serde::{Deserialize, Serialize};
use serde_json::{self, Error};
use serde_wasm_bindgen;
use std::{fmt, string};
use sube::{
    http::{Backend as HttpBackend, Url},
    meta::Meta,
    meta_ext::Pallet,
    rpc, sube,
    util::to_camel,
    Backend, Error as SubeError, ExtrinicBody, JsonValue, Response,
};
// use sp_core::{crypto::Ss58Codec, hexdisplay::AsBytesRef};
use util::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
extern crate console_error_panic_hook;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn chain_string_to_url(chain: &str) -> Result<Url> {
    let chain = if !chain.starts_with("ws://")
        && !chain.starts_with("wss://")
        && !chain.starts_with("http://")
        && !chain.starts_with("https://")
    {
        ["wss", &chain].join("://")
    } else {
        chain.into()
    };

    let mut url = Url::parse(&chain)?;
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

#[derive(Serialize, Deserialize)]
struct ExtrinicBodyWithFrom {
    from: Vec<u8>,
    call: ExtrinicBody<JsonValue>,
}

#[wasm_bindgen]
pub async fn sube_js(
    url: &str,
    params: JsValue,
    signer: Option<js_sys::Function>,
) -> Result<JsValue> {
    console_log::init_with_level(Level::max());
    console_error_panic_hook::set_once();

    if params.is_undefined() {
        let response = sube!(url)
            .await
            .map_err(|e| JsError::new(&format!("Error querying: {:?}", &e.to_string())))?;

        let value = match response {
            v @ Response::Value(_) | v @ Response::Meta(_) | v @ Response::Registry(_) => {
                let value = serde_wasm_bindgen::to_value(&v)
                    .map_err(|_| JsError::new("failed to serialize response"))?;
                Ok(value)
            }
            _ => Err(JsError::new("Nonve value at query")),
        }?;

        return Ok(value);
    }

    let mut extrinsic_value: ExtrinicBodyWithFrom = serde_wasm_bindgen::from_value(params)?;

    extrinsic_value.call.body = decode_addresses(&extrinsic_value.call.body);

    let value = sube!(url => {
        signer: move |message: &[u8]| unsafe {
            let response: JsValue = signer
                .clone()
                .ok_or(SubeError::BadInput)?
                .call1(
                    &JsValue::null(),
                    &JsValue::from(js_sys::Uint8Array::from(message)),
                )
                .map_err(|_| SubeError::Signing)?;

            let vec: Vec<u8> = serde_wasm_bindgen::from_value(response)
                .map_err(|_| SubeError::Encode("Unknown value to decode".into()))?;

            let buffer: [u8; 64] = vec.try_into().expect("slice with incorrect length");

            Ok(buffer)
        },
        sender: extrinsic_value.from.as_slice(),
        body: extrinsic_value.call,
    })
    .await
    .map_err(|e| JsError::new(&format!("Error trying: {:?}", e.to_string())))?;

    match value {
        Response::Void => Ok(JsValue::null()),
        _ => Err(JsError::new("Unknown Response")),
    }
}
