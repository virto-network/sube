use crate::io::{self, Input, InputStream, Output, OutputSink};
use core::cell::OnceCell;
use futures_util::{never::Never, sink::unfold, Sink, Stream};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    js_sys::{global, Object, Reflect},
    DedicatedWorkerGlobalScope, MessageEvent,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn start() {
    wasm_logger::init(Default::default());
    log::debug!("worker started");
    if let Err(_) = crate::run().await {
        log::warn!("vos client setup failed");
    }
}

pub fn setup_io() -> (impl InputStream, impl OutputSink) {
    const ON_MSG: OnceCell<JsValue> = OnceCell::new();
    async fn process_worker_message(
        sender: async_channel::Sender<Input>,
        event: MessageEvent,
    ) -> Result<(), JsValue> {
        // let Ok(message) = event.data().dyn_into::<Object>() else {
        //     return Ok(());
        // };
        let input: Input = serde_wasm_bindgen::from_value(event.data())?;
        // let id = Reflect::get(&message, &"id".into())?
        //     .as_f64()
        //     .ok_or("Missing msg id")?
        //     .round() as u32;
        // let cmd = Reflect::get(&message, &"cmd".into())?
        //     .as_string()
        //     .ok_or("Invalid command")?;

        sender
            .send(input)
            .await
            .map_err(|e| format!("processing: {e}").into())
    }

    let worker = global()
        .dyn_into::<DedicatedWorkerGlobalScope>()
        .expect("worker");
    let (sender, cmd_stream) = async_channel::unbounded();

    let on_msg = ON_MSG;
    let on_msg = on_msg.get_or_init(move || {
        let cb = Closure::wrap(Box::new(move |event| {
            let s = sender.clone();
            spawn_local(async move {
                if let Err(err) = process_worker_message(s.clone(), event).await {
                    log::error!(
                        "{}",
                        &err.as_string()
                            .unwrap_or_else(|| "incoming message error".to_string())
                    )
                }
            })
        }) as Box<dyn FnMut(MessageEvent)>);
        cb.into_js_value()
    });
    worker.set_onmessage(Some(on_msg.unchecked_ref()));

    let reply_sender = unfold((), |_, out: io::Result| async move {
        let worker = global().unchecked_into::<DedicatedWorkerGlobalScope>();
        let out = serde_wasm_bindgen::to_value(&out).expect("output serialized");
        worker.post_message(&out).expect("output sent");
        Ok::<_, ()>(())
    });

    (cmd_stream, reply_sender)
}
