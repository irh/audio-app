use super::app::AUDIO_STREAM;
use audio_module::{FloatParameter, Parameter, PushMessage, ToProcessor};
use dioxus::prelude::*;
use dioxus_primitives::slider::{
    self, SliderRangeProps, SliderThumbProps, SliderTrackProps, SliderValue,
};

#[component]
pub fn ParameterSlider(parameter: FloatParameter) -> Element {
    let id = parameter.id();
    let value_converter = parameter.value_converter();
    let string_converter = parameter.string_converter();

    // Convert the name into a `String` once when the slider is first created.
    let name = use_hook(|| parameter.name().to_string());

    // Create a signal based on the parameter value so that we can respond to changes.
    let mut value = use_signal(|| parameter.value);

    // Derive a linear value for use by the slider.
    let linear_value = use_memo({
        let value_converter = value_converter.clone();
        move || value_converter.user_to_linear(value()) as f64
    });

    // Calculate the default linear value once when the slider is first created.
    let default_linear_value =
        use_hook(|| value_converter.user_to_linear(parameter.default_user_value()) as f64);

    // Send updated values to the processor.
    use_effect(move || {
        if let Some(to_processor) = AUDIO_STREAM().map(|stream| stream.to_processor()) {
            to_processor.push(ToProcessor::SetParameter(id, value()));
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./slider.css") }

        div {
            class: "slider-name",
            "{name}"
        }

        div {
            class: "slider-and-value",

            slider::Slider {
                class: "slider",
                value: Some(SliderValue::Single(linear_value())),
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default_value: SliderValue::Single(default_linear_value as f64),
                on_value_change: move |new_value| match new_value{
                    SliderValue::Single(new_value) => {
                        *value.write() = value_converter.linear_to_user(new_value as f32);
                    }
                },
                label: name,

                SliderTrack {
                    SliderRange {}
                    SliderThumb {}
                }
            }

            { string_converter.to_string_with_unit(value()) }
        }
    }
}

#[component]
pub fn SliderTrack(props: SliderTrackProps) -> Element {
    rsx! {
        slider::SliderTrack {
            class: "slider-track",
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn SliderRange(props: SliderRangeProps) -> Element {
    rsx! {
        slider::SliderRange {
            class: "slider-range",
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn SliderThumb(props: SliderThumbProps) -> Element {
    rsx! {
        slider::SliderThumb {
            class: "slider-thumb",
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}
