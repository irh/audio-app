use super::{
    slider::ParameterSlider,
    toggle::{ParameterToggle, Toggle},
};
use audio_stream::AudioStream;
use dioxus::prelude::*;
use freeverb_module::{FreeverbModule, FreeverbParameters};
use std::sync::Arc;

const MAIN_CSS: Asset = asset!("/assets/main.css");

pub type FreeverbStream = AudioStream<FreeverbModule>;
pub static AUDIO_STREAM: GlobalSignal<Option<Arc<FreeverbStream>>> = Signal::global(|| None);

#[component]
pub fn App() -> Element {
    #[cfg(feature = "web")]
    {
        // Initialize the audio stream's processor Wasm.
        use_future(async || {
            if let Err(error) = audio_stream::initialize_audio(
                "./freeverb_bg.wasm",
                "./freeverb.js",
                "./audio_stream_worklet.js",
            )
            .await
            {
                error!("Failed to initialize audio: {error:?}");
            }
        });
    }

    // Load the parameters when the app is first initialized.
    let parameters = use_hook(FreeverbParameters::default);

    // Create a signal for enabling or disabling the audio stream.
    let mut audio_enabled = use_signal(|| false);
    // Create or destroy the audio stream when `audio_enabled` changes.
    use_effect(move || {
        if audio_enabled() {
            match AudioStream::new() {
                Ok(stream) => *AUDIO_STREAM.write() = Some(Arc::new(stream)),
                Err(error) => {
                    error!("Failed to create audio stream: {error}");
                    *audio_enabled.write() = false;
                }
            }
        } else {
            *AUDIO_STREAM.write() = None;
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        div {
            class: "header",

            div {
                class: "header-text",
                "Freeverb"
            }

            Toggle {
                pressed: audio_enabled(),
                on_pressed_change: move |pressed| *audio_enabled.write() = pressed,

                "Enable Audio"
            }
        }

        ParameterSlider { parameter: parameters.dampening }
        ParameterSlider { parameter: parameters.width }
        ParameterSlider { parameter: parameters.room_size }
        ParameterToggle { parameter: parameters.freeze }
        ParameterSlider { parameter: parameters.dry }
        ParameterSlider { parameter: parameters.wet }
    }
}
