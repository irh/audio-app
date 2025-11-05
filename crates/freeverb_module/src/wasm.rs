#![cfg(target_arch = "wasm32")]

use crate::FreeverbProcessor;
use audio_module::{AudioProcessor, ToProcessor};
use js_sys::Float32Array;
use wasm_bindgen::{JsValue, prelude::*};

#[wasm_bindgen]
struct Processor {
    processor: FreeverbProcessor<f32>,
    // A buffer for interleaving / deinterleaving the audio worklet's buffers
    buffer: [f32; 128 * 2],
}

#[wasm_bindgen]
impl Processor {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: usize) -> Self {
        Self {
            processor: FreeverbProcessor::new(sample_rate),
            buffer: [0.0; _],
        }
    }

    #[wasm_bindgen]
    pub fn set_parameter(&mut self, parameter_id: usize, value: f32) {
        self.processor
            .receive_message(ToProcessor::SetParameter(parameter_id, value));
    }

    #[wasm_bindgen]
    pub fn process(
        &mut self,
        input_l: Float32Array,
        input_r: Float32Array,
        output_l: Float32Array,
        output_r: Float32Array,
        on_message: &js_sys::Function,
    ) {
        for (i, buffer_frame) in self.buffer.chunks_exact_mut(2).enumerate() {
            buffer_frame[0] = input_l.get_index(i as u32);
            buffer_frame[1] = input_r.get_index(i as u32);
        }

        self.processor
            .process_buffer(&mut self.buffer, 2, |message| {
                if let Ok(js_message) = serde_wasm_bindgen::to_value(&message) {
                    on_message.call1(&JsValue::null(), &js_message).ok();
                }
            });

        for (i, buffer_frame) in self.buffer.chunks_exact_mut(2).enumerate() {
            output_l.set_index(i as u32, buffer_frame[0]);
            output_r.set_index(i as u32, buffer_frame[1]);
        }
    }
}
