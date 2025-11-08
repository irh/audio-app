mod components;

use crate::components::{ParameterSlider, ParameterToggle, Toggle};
use audio_stream::AudioStream;
use dioxus::prelude::*;
use freeverb_module::{FreeverbModule, FreeverbParameters};
use std::sync::Arc;

const MAIN_CSS: Asset = asset!("/assets/main.css");

type FreeverbStream = AudioStream<FreeverbModule>;
static AUDIO_STREAM: GlobalSignal<Option<Arc<FreeverbStream>>> = Signal::global(|| None);

fn main() {
    dioxus::logger::initialize_default();

    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug) // limit log level
                .with_tag("freeverb") // logs will show under mytag tag
                .with_filter(
                    // configure messages for specific crate
                    android_logger::FilterBuilder::new()
                        .parse("debug,hello::crate=error")
                        .build(),
                ),
        );
    }

    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
            Config::new().with_window(
                WindowBuilder::new()
                   .with_title(env!("PRODUCT_NAME"))
                   .with_inner_size(LogicalSize::new(310, 500))
            )
        })
        .launch(App);
}

#[component]
fn App() -> Element {
    #[cfg(feature = "web")]
    {
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

    let parameters = use_hook(FreeverbParameters::default);

    let mut audio_enabled = use_signal(|| false);
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
