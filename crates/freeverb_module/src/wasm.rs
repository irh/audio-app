#![cfg(target_arch = "wasm32")]

use crate::{FreeverbProcessor, FromFreeverb};
use audio_module::{AudioProcessor, PopMessage, PushMessage, ToProcessor};
use js_sys::Float32Array;
use std::{cell::RefCell, collections::VecDeque};
use wasm_bindgen::{JsValue, prelude::*};

#[wasm_bindgen]
struct Processor {
    processor: FreeverbProcessor<f32>,
    to_processor: ToProcessorMessages,
    from_processor: FromProcessorMessages,
    // A buffer for interleaving / deinterleaving the audio worklet's buffers
    buffer: [f32; 128 * 2],
}

#[wasm_bindgen]
impl Processor {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: usize) -> Self {
        Self {
            processor: FreeverbProcessor::new(sample_rate),
            to_processor: Default::default(),
            from_processor: Default::default(),
            buffer: [0.0; _],
        }
    }

    #[wasm_bindgen]
    pub fn set_parameter(&mut self, parameter_id: usize, value: f32) {
        self.to_processor
            .messages
            .borrow_mut()
            .push_back(ToProcessor::SetParameter(parameter_id, value));
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
        // Interleave the input buffers into the process buffer
        for (i, buffer_frame) in self.buffer.chunks_exact_mut(2).enumerate() {
            buffer_frame[0] = input_l.get_index(i as u32);
            buffer_frame[1] = input_r.get_index(i as u32);
        }

        self.processor.process_buffer(
            &mut self.buffer,
            2,
            &self.to_processor,
            &self.from_processor,
        );

        // Deinterleave the process buffer into the output buffers
        for (i, buffer_frame) in self.buffer.chunks_exact_mut(2).enumerate() {
            output_l.set_index(i as u32, buffer_frame[0]);
            output_r.set_index(i as u32, buffer_frame[1]);
        }

        let mut messages = self.from_processor.messages.borrow_mut();
        while let Some(message) = messages.pop_front() {
            if let Ok(js_message) = serde_wasm_bindgen::to_value(&message) {
                on_message.call1(&JsValue::null(), &js_message).ok();
            }
        }
    }
}

#[derive(Default)]
struct ToProcessorMessages {
    messages: RefCell<VecDeque<ToProcessor>>,
}

impl PopMessage<ToProcessor> for ToProcessorMessages {
    fn pop(&self) -> Option<ToProcessor> {
        self.messages.borrow_mut().pop_front()
    }
}

#[derive(Default)]
struct FromProcessorMessages {
    messages: RefCell<VecDeque<FromFreeverb>>,
}

impl PushMessage<FromFreeverb> for FromProcessorMessages {
    fn push(&self, message: FromFreeverb) -> bool {
        self.messages.borrow_mut().push_back(message);
        true
    }
}
