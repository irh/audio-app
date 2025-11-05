#![cfg(target_arch = "wasm32")]

use app_egui::App;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[wasm_bindgen]
impl WebHandle {
    #[expect(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();

        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    #[wasm_bindgen]
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_cc| {
                    let app = App::new().map_err(|e| e.into_boxed_dyn_error())?;
                    Ok(Box::new(app))
                }),
            )
            .await
    }
}

#[wasm_bindgen]
pub async fn initialize_audio() -> Result<(), wasm_bindgen::JsValue> {
    audio_stream::initialize_audio(
        "./freeverb_bg.wasm",
        "./freeverb.js",
        "./audio_stream_worklet.js",
    )
    .await
}
