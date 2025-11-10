use audio_module::{AudioModule, AudioProcessor, PopMessage, PushMessage, ToProcessor};
use crossbeam_channel::Receiver;
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde::Deserialize;
use std::{cell::RefCell, marker::PhantomData};
use thiserror::Error;
use wasm_bindgen::{JsValue, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, MediaStream, MediaStreamConstraints,
    Request, RequestInit, RequestMode, Response, console, window,
};

pub const FRAMES_PER_BUFFER: usize = 128;

thread_local! {
    static AUDIO_STATE: RefCell<Option<AudioState>> = RefCell::new(None);
}

#[derive(Clone)]
struct AudioState {
    audio_context: AudioContext,
    processor_node: AudioWorkletNode,
    sample_rate: usize,
    message_receiver: Receiver<JsValue>,
}

/// Sets up the audio context with an `AudioWorkletNode`
///
/// `worklet_js_path` should be the path to a copy of `./wasm/audio_stream_worklet.js`.
///   - This contains the `AudioWorkletNode` that will process input from the microphone.
/// `wasm_path` and `wasm_glue_path` should be the paths to a wasm module along with its glue `.js`
///   that will be passed into the `AudioWorkletNode`.
///   - The following functions should be exported from the wasm:
///     - `create_processor(sample_rate: usize) -> *mut Processor`:
///       - Creates an instance of a processor.
///     - `set_parameter(*mut Processor, id: usize, value: f32)`:
///       - Sets a parameter by id.
///     - `process(*mut Processor,
///                input_l: *const f32, input_r: *const f32, output_l: *mut f32, output_r: *mut f32,
///                sample_count: usize)`:
///       - Processes frames provided in deinterleaved buffers.
///     - `createBuffer(size: usize) -> *mut f32`:
///       - Creates a buffer for use by the process function.
pub async fn initialize_audio(
    wasm_path: &str,
    wasm_glue_path: &str,
    worklet_js_path: &str,
) -> std::result::Result<(), JsValue> {
    let window = window().ok_or("missing global window")?;

    // Fetch the audio worklet `.wasm`
    let fetch_options = RequestInit::new();
    fetch_options.set_method("GET");
    fetch_options.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(&wasm_path, &fetch_options)?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await?
        .dyn_into()?;
    // Place the fetched wasm in a buffer that can be passed to the worklet
    let wasm_buffer = Uint8Array::new(&JsFuture::from(response.array_buffer()?).await?);

    // Fetch the wasm's glue code
    let request = Request::new_with_str_and_init(&wasm_glue_path, &fetch_options)?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await?
        .dyn_into()?;
    let glue_js = JsFuture::from(response.text()?).await?;

    // Create the audio context and add the audio worklet module
    let audio_context = AudioContext::new()?;
    JsFuture::from(
        audio_context
            .audio_worklet()?
            .add_module(&worklet_js_path)?,
    )
    .await?;

    // Get access to the mic
    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::TRUE);
    constraints.set_video(&JsValue::FALSE);

    let stream_promise = window
        .navigator()
        .media_devices()?
        .get_user_media_with_constraints(&constraints)?;
    let stream: MediaStream = JsFuture::from(stream_promise).await?.dyn_into()?;

    let mic = audio_context.create_media_stream_source(&stream)?;

    // Create the processor worklet, passing the wasm buffer and sample rate to the constructor
    let node_options = AudioWorkletNodeOptions::new();

    let channel_count = Array::new();
    channel_count.push(&JsValue::from(2));
    node_options.set_output_channel_count(&channel_count);

    let sample_rate = audio_context.sample_rate();
    let processor_options = Object::new();
    Reflect::set(
        &processor_options,
        &JsValue::from_str("wasmBuffer"),
        &wasm_buffer.buffer(),
    )?;
    Reflect::set(&processor_options, &JsValue::from_str("wasmGlue"), &glue_js)?;
    Reflect::set(
        &processor_options,
        &JsValue::from_str("sampleRate"),
        &JsValue::from(sample_rate),
    )?;
    node_options.set_processor_options(Some(&processor_options));

    let processor_node =
        AudioWorkletNode::new_with_options(&audio_context, "AudioStreamWorklet", &node_options)?;

    // Connect the nodes: mic -> worklet -> destination
    mic.connect_with_audio_node(&processor_node)?;
    processor_node.connect_with_audio_node(&audio_context.destination())?;

    // Suspend the audio context now that it's initialized, wait for the user to enable it
    JsFuture::from(audio_context.suspend()?).await?;

    let (message_sender, message_receiver) = crossbeam_channel::bounded(1024);

    // Move producer into the message handler closure
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let data = event.data();

        if message_sender.try_send(data).is_err() {
            console::warn_1(&"Ring buffer full, dropping message".into());
        }
    }) as Box<dyn FnMut(_)>);

    processor_node
        .port()?
        .set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    onmessage.forget();

    AUDIO_STATE.with(|state| {
        *state.borrow_mut() = Some(AudioState {
            audio_context,
            processor_node,
            sample_rate: sample_rate as usize,
            message_receiver,
        })
    });

    Ok(())
}

#[derive(Clone)]
pub struct AudioStream<M> {
    state: AudioState,
    _module: PhantomData<M>,
}

impl<M: AudioModule> AudioStream<M> {
    pub fn new() -> Result<Self> {
        let Some(state) = AUDIO_STATE.with(|ctx| ctx.borrow().clone()) else {
            return Err(Error::AudioUninitialized);
        };

        let audio_context = state.audio_context.clone();
        spawn_local(async move {
            match audio_context.resume() {
                Ok(promise) => match JsFuture::from(promise).await {
                    Ok(_) => console::log_1(&"Audio context resumed".into()),
                    Err(error) => console::error_1(&error),
                },
                Err(error) => console::error_1(&error),
            }
        });

        Ok(Self {
            state,
            _module: PhantomData,
        })
    }

    pub fn to_processor(&self) -> ToProcessorSender {
        ToProcessorSender {
            processor_node: self.state.processor_node.clone(),
        }
    }

    pub fn from_processor(&self) -> FromProcessorReceiver<M::Processor> {
        FromProcessorReceiver {
            message_receiver: self.state.message_receiver.clone(),
            _processor: PhantomData,
        }
    }

    pub fn sample_rate(&self) -> usize {
        self.state.sample_rate
    }
}

impl<M> Drop for AudioStream<M> {
    fn drop(&mut self) {
        let audio_context = self.state.audio_context.clone();
        spawn_local(async move {
            match audio_context.suspend() {
                Ok(promise) => match JsFuture::from(promise).await {
                    Ok(_) => console::log_1(&"Audio context suspended".into()),
                    Err(error) => console::error_1(&error),
                },
                Err(error) => console::error_1(&error),
            }
        });
    }
}

#[derive(Clone, PartialEq)]
pub struct ToProcessorSender {
    processor_node: AudioWorkletNode,
}

impl PushMessage<ToProcessor> for ToProcessorSender {
    fn push(&self, message: ToProcessor) -> bool {
        match message {
            ToProcessor::SetParameter(id, value) => match self.processor_node.port() {
                Ok(port) => {
                    let message = Object::new();
                    Reflect::set(&message, &"id".into(), &id.into()).ok();
                    Reflect::set(&message, &"value".into(), &value.into()).ok();
                    if let Err(error) = port.post_message(&message) {
                        console::error_1(&error);
                        false
                    } else {
                        true
                    }
                }
                Err(error) => {
                    console::error_1(&error);
                    false
                }
            },
            _ => true,
        }
    }
}

#[derive(Clone)]
pub struct FromProcessorReceiver<P> {
    message_receiver: Receiver<JsValue>,
    _processor: PhantomData<P>,
}

impl<P> PopMessage<P::OutputMessage> for FromProcessorReceiver<P>
where
    P: AudioProcessor,
    P::OutputMessage: for<'de> Deserialize<'de>,
{
    fn pop(&self) -> Option<P::OutputMessage> {
        if let Ok(message) = self.message_receiver.try_recv() {
            match serde_wasm_bindgen::from_value(message) {
                Ok(result) => Some(result),
                Err(error) => {
                    console::error_1(&error.to_string().into());
                    None
                }
            }
        } else {
            None
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Audio is uninitialized (did you call `initialize_audio`?)")]
    AudioUninitialized,
}

pub type Result<T> = std::result::Result<T, Error>;
