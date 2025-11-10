use super::{
    slider::ParameterSlider,
    toggle::{ParameterToggle, Toggle},
};
use audio_stream::AudioStream;
use dioxus::prelude::*;
use freeverb_module::{FreeverbModule, FreeverbParameters};
use std::sync::Arc;

const MAIN_CSS: Asset = asset!("/assets/main.css");

// A wrapper for the audio stream that can be used as a signal
#[derive(Clone)]
struct FreeverbStream(Arc<AudioStream<FreeverbModule>>);

impl PartialEq for FreeverbStream {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

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
    let audio_stream = use_memo(move || {
        if audio_enabled() {
            match AudioStream::new() {
                Ok(stream) => Some(FreeverbStream(Arc::new(stream))),
                Err(error) => {
                    error!("Failed to create audio stream: {error}");
                    *audio_enabled.write() = false;
                    None
                }
            }
        } else {
            None
        }
    });

    // Derive a signal from the audio stream that provides the to_processor sender
    let to_processor = use_memo(move || {
        audio_stream()
            .as_ref()
            .map(|stream| stream.0.to_processor())
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

        ParameterSlider { parameter: parameters.dampening, to_processor: to_processor }
        ParameterSlider { parameter: parameters.width, to_processor: to_processor }
        ParameterSlider { parameter: parameters.room_size, to_processor: to_processor }
        ParameterToggle { parameter: parameters.freeze, to_processor: to_processor }
        ParameterSlider { parameter: parameters.dry, to_processor: to_processor }
        ParameterSlider { parameter: parameters.wet, to_processor: to_processor }
    }
}
