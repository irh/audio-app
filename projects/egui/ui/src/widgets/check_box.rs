use audio_module::{BoolParameter, Parameter, PushMessage, ToProcessor};
use egui::{Response, Ui, Widget};

pub struct Checkbox<'a, T: PushMessage> {
    parameter: &'a mut BoolParameter,
    to_processor: &'a mut Option<T>,
}

impl<'a, T: PushMessage> Checkbox<'a, T> {
    pub fn new(parameter: &'a mut BoolParameter, to_processor: &'a mut Option<T>) -> Self {
        Self {
            parameter,
            to_processor,
        }
    }
}

impl<'a, T: PushMessage> Widget for Checkbox<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut value = self.parameter.value;
        let response = ui.checkbox(&mut value, self.parameter.name().as_str());

        if response.changed() {
            self.parameter.value = value;
            if let Some(commands) = self.to_processor {
                commands.push(ToProcessor::SetParameter(
                    self.parameter.id(),
                    if value { 1.0 } else { 0.0 },
                ));
            }
        }

        response
    }
}
