mod phase_scope;

use crate::app::Message;
use audio_module::{BoolParameter, FloatParameter, Parameter};
use freeverb_module::{FreeverbParameterId, FreeverbParameters};
use iced::widget::{checkbox, column, container, row, slider, text};
use iced::{Alignment, Element, Fill};

pub use self::phase_scope::PhaseScope;

pub fn freeverb_parameters(parameters: &FreeverbParameters) -> Element<'_, Message> {
    column![
        parameter_slider(&parameters.dampening, FreeverbParameterId::Dampening),
        parameter_slider(&parameters.width, FreeverbParameterId::Width),
        parameter_slider(&parameters.room_size, FreeverbParameterId::RoomSize),
        parameter_toggle(&parameters.freeze, FreeverbParameterId::Freeze),
        parameter_slider(&parameters.dry, FreeverbParameterId::Dry),
        parameter_slider(&parameters.wet, FreeverbParameterId::Wet),
    ]
    .width(300.0)
    .spacing(20)
    .into()
}

pub fn parameter_slider(
    parameter: &FloatParameter,
    id: FreeverbParameterId,
) -> Element<'_, Message> {
    let name = parameter.name();
    let value = parameter.value;
    let string_converter = parameter.string_converter();
    let default_value = parameter.default_user_value();

    let caption = text(name.to_string());

    let slider = container(
        slider(0.0..=1.0, value, move |new_value| {
            Message::SetFloat(id, new_value)
        })
        .default(default_value)
        .step(0.01)
        .shift_step(0.1),
    );

    let value_text = text(string_converter.to_string_with_unit(value));
    let slider_with_value = row![slider, value_text].width(Fill).spacing(10);

    column![caption, slider_with_value]
        .width(Fill)
        .align_x(Alignment::Start)
        .spacing(10)
        .into()
}

pub fn parameter_toggle(
    parameter: &BoolParameter,
    id: FreeverbParameterId,
) -> Element<'_, Message> {
    let name = parameter.name().to_string();
    let value = parameter.value;

    checkbox(name, value)
        .on_toggle(move |new_value| Message::SetBool(id, new_value))
        .into()
}
