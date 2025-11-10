use audio_module::{BoolParameter, Parameter, PushMessage, ToProcessor};
use audio_stream::ToProcessorSender;
use dioxus::prelude::*;
use dioxus_primitives::toggle::{self, ToggleProps};

#[component]
pub fn ParameterToggle(
    parameter: BoolParameter,
    to_processor: ReadSignal<Option<ToProcessorSender>>,
) -> Element {
    let mut value = use_signal(|| parameter.value);

    let name = use_hook(|| parameter.name().to_string());
    let id = parameter.id();

    use_effect(move || {
        if let Some(to_processor) = to_processor() {
            let value = if value() { 1.0 } else { 0.0 };
            to_processor.push(ToProcessor::SetParameter(id, value));
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./toggle.css") }

        Toggle {
            pressed: Some(value()),
            default_pressed: parameter.default_user_value() != 0.0,
            on_pressed_change: move |pressed| {
                *value.write() = pressed;
            },

            "{name}"
        }
    }
}

#[component]
pub fn Toggle(props: ToggleProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./toggle.css") }

        toggle::Toggle {
            class: "toggle",
            pressed: props.pressed,
            default_pressed: props.default_pressed,
            on_pressed_change: props.on_pressed_change,
            onmounted: props.onmounted,
            onfocus: props.onfocus,
            onkeydown: props.onkeydown,
            attributes: props.attributes,
            children: props.children,
        }
    }
}
