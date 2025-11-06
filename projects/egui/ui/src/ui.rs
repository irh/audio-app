use std::collections::VecDeque;

use crate::widgets::{Checkbox, FloatSlider, PhaseScope};
use audio_module::{PopMessage, PushMessage, ToProcessor};
use egui::{Response, Ui, Widget, vec2};
use freeverb_module::{FreeverbParameters, FromFreeverb};

#[derive(Default)]
pub struct FreeverbUiState {
    pub parameters: FreeverbParameters,
    pub scope_frames: VecDeque<(f32, f32)>,
    pub sample_rate: usize,
}

impl FreeverbUiState {
    pub fn receive_processor_messages(&mut self, from_processor: &impl PopMessage<FromFreeverb>) {
        while let Some(message) = from_processor.pop() {
            match message {
                FromFreeverb::ScopeBuffer(buffer) => {
                    self.scope_frames.extend(buffer.iter());
                    dbg!(self.scope_frames.len());

                    // Keep the most recent N frames
                    let scope_tail_duration = 0.2;
                    let scope_frame_count =
                        (scope_tail_duration * self.sample_rate as f64) as usize;
                    let frames_to_drop = self.scope_frames.len().saturating_sub(scope_frame_count);

                    // `VecDeque::truncate_front` can be used here once it's stable.
                    self.scope_frames.drain(0..frames_to_drop);
                }
            }
        }
    }
}

pub struct FreeverbUi<'a, T: PushMessage<ToProcessor>> {
    state: &'a mut FreeverbUiState,
    to_processor: Option<T>,
}

impl<'a, T: PushMessage<ToProcessor>> FreeverbUi<'a, T> {
    pub fn new(state: &'a mut FreeverbUiState, to_processor: Option<T>) -> Self {
        Self {
            state,
            to_processor,
        }
    }
}

impl<'a, T: PushMessage<ToProcessor>> Widget for FreeverbUi<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let parameters = &mut self.state.parameters;

        let contents = |ui: &mut Ui| {
            // Parameters
            ui.vertical(|ui| {
                ui.add(FloatSlider::new(
                    &mut parameters.dampening,
                    &self.to_processor,
                ));
                ui.add(FloatSlider::new(&mut parameters.width, &self.to_processor));
                ui.add(FloatSlider::new(
                    &mut parameters.room_size,
                    &self.to_processor,
                ));
                ui.add(Checkbox::new(&mut parameters.freeze, &self.to_processor));
                ui.add(FloatSlider::new(&mut parameters.dry, &self.to_processor));
                ui.add(FloatSlider::new(&mut parameters.wet, &self.to_processor));
            });

            // Scope
            ui.vertical(|ui| {
                let size = ui.available_size();
                ui.add_sized(
                    vec2(size.y, size.y),
                    PhaseScope::new(
                        self.to_processor.is_some(),
                        self.state.scope_frames.iter().cloned(),
                    ),
                );
            });
        };

        if ui.available_width() > ui.available_height() {
            ui.horizontal(contents)
        } else {
            ui.vertical(contents)
        }
        .response
    }
}
