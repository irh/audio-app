use audio_module::{FloatParameter, Parameter, PushMessage, ToProcessor};
use egui::{self, Response, Ui, Widget};

pub struct FloatSlider<'a, T: PushMessage> {
    parameter: &'a mut FloatParameter,
    to_processor: &'a mut Option<T>,
}

impl<'a, T: PushMessage> FloatSlider<'a, T> {
    pub fn new(parameter: &'a mut FloatParameter, to_processor: &'a mut Option<T>) -> Self {
        Self {
            parameter,
            to_processor,
        }
    }
}

impl<'a, T: PushMessage> Widget for FloatSlider<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            let value_converter = self.parameter.value_converter();
            let string_converter = self.parameter.string_converter();
            let id = self.parameter.id();

            ui.label(self.parameter.name().as_str());

            let response = ui.add({
                let mut slider = egui::Slider::new(
                    &mut self.parameter.value,
                    value_converter.min()..=value_converter.max(),
                )
                .custom_formatter(|n, _| string_converter.to_string(n as f32))
                .custom_parser(|s| string_converter.to_f32(s).map(|n| n as f64));

                if let Some(unit) = string_converter.unit() {
                    slider = slider.suffix(format!(" {unit}"));
                }

                slider
            });

            if let Some(to_processor) = self.to_processor {
                if response.drag_started() {
                    to_processor.push(ToProcessor::BeginEdit(id));
                }

                if response.changed() {
                    to_processor.push(ToProcessor::SetParameter(id, self.parameter.value));
                }

                if response.drag_stopped() {
                    to_processor.push(ToProcessor::EndEdit(id));
                }
            }

            response
        })
        .response
    }
}
