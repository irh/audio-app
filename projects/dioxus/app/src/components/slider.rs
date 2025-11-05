use crate::AUDIO_STREAM;
use audio_module::{FloatParameter, Parameter, PushMessage, ToProcessor};
use dioxus::prelude::*;
use dioxus_primitives::slider::{
    self, SliderRangeProps, SliderThumbProps, SliderTrackProps, SliderValue,
};

#[component]
pub fn ParameterSlider(parameter: FloatParameter) -> Element {
    let mut value = use_signal(|| parameter.value);

    let name = use_hook(|| parameter.name().to_string());
    let id = parameter.id();
    let value_converter = parameter.value_converter();
    let string_converter = parameter.string_converter();

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
                value: Some(SliderValue::Single(value() as f64)),
                min: value_converter.min() as f64,
                max: value_converter.max() as f64,
                step: 0.01,
                default_value: SliderValue::Single(parameter.default_user_value() as f64),
                on_value_change: move |new_value| match new_value{
                    SliderValue::Single(new_value) => {
                        *value.write() = new_value as f32;
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
