#![allow(clippy::future_not_send)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

fn warn(msg: &str) {
    web_sys::console::warn_1(&msg.into());
}

pub async fn play_note(string: u8, fret: u8) {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "string": string,
        "fret": fret,
    }))
    .unwrap();
    let result = invoke("play_note", args).await;
    if let Some(err) = result.as_string() {
        if !err.is_empty() {
            warn(&format!("play_note failed: {}", err));
        }
    }
}

pub async fn stop_note(string: u8, fret: u8) {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "string": string,
        "fret": fret,
    }))
    .unwrap();
    let result = invoke("stop_note", args).await;
    if let Some(err) = result.as_string() {
        if !err.is_empty() {
            warn(&format!("stop_note failed: {}", err));
        }
    }
}

pub async fn stop_all_notes() {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
    let result = invoke("stop_all_notes", args).await;
    if let Some(err) = result.as_string() {
        if !err.is_empty() {
            warn(&format!("stop_all_notes failed: {}", err));
        }
    }
}

pub async fn load_exercises() -> Option<String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
    let result = invoke("load_exercises", args).await;
    serde_wasm_bindgen::from_value(result).ok()
}
