#![allow(clippy::future_not_send)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

pub async fn play_note(string: u8, fret: u8) {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "string": string,
        "fret": fret,
    }))
    .unwrap();
    invoke("play_note", args).await;
}

pub async fn stop_note(string: u8, fret: u8) {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "string": string,
        "fret": fret,
    }))
    .unwrap();
    invoke("stop_note", args).await;
}

pub async fn stop_all_notes() {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
    invoke("stop_all_notes", args).await;
}

pub async fn load_exercises() -> Option<String> {
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
    let result = invoke("load_exercises", args).await;
    serde_wasm_bindgen::from_value(result).ok()
}
