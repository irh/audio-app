mod phase_scope;

use crate::app::Message;
use audio_module::{BoolParameter, FloatParameter, Parameter};
use vizia::prelude::*;

pub use phase_scope::{PhaseScope, ScopeFrames};

pub fn parameter_slider(cx: &mut Context, parameter_lens: impl Lens<Target = FloatParameter>) {
    let parameter = parameter_lens.get(cx);
    let name = parameter.name();
    let id = parameter.id();
    let string_converter = parameter.string_converter();
    let value = parameter_lens.map_ref(|p| &p.value);

    VStack::new(cx, |cx| {
        Label::new(cx, name.as_str()).alignment(Alignment::Left);
        HStack::new(cx, |cx| {
            Slider::new(cx, value).on_change(move |cx, value| {
                cx.emit(Message::SetFloat(id.try_into().unwrap(), value))
            });
            Label::new(
                cx,
                value.map(move |value| string_converter.to_string_with_unit(*value)),
            )
            .alignment(Alignment::Center)
            .width(Pixels(50.0));
        })
        .alignment(Alignment::Left)
        .height(Auto)
        .horizontal_gap(Pixels(5.0));
    })
    .size(Auto)
    .width(Stretch(1.0))
    .alignment(Alignment::Left)
    .vertical_gap(Pixels(5.0));
}

pub fn parameter_toggle(cx: &mut Context, parameter_lens: impl Lens<Target = BoolParameter>) {
    let parameter = parameter_lens.get(cx);
    let id = parameter.id();

    HStack::new(cx, |cx| {
        ToggleButton::new(cx, parameter_lens.map_ref(|p| &p.value), {
            let name = parameter.name().to_string();
            move |cx| Label::new(cx, &name)
        })
        .on_toggle(move |cx| cx.emit(Message::ToggleBool(id.try_into().unwrap())));
    })
    .size(Auto)
    .horizontal_gap(Pixels(10.0))
    .alignment(Alignment::Center);
}
