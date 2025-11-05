use anyhow::Result;
use audio_module::{PopMessage, PushMessage, ToProcessor};
use audio_stream::AudioStream;
use eframe::{
    Frame,
    egui::{self, Align, CentralPanel, Layout, RichText, TextStyle},
};
use freeverb_module::{FreeverbModule, FreeverbParameterId, FromFreeverb};
use ui_egui::{FreeverbUi, FreeverbUiState};

pub struct App {
    ui_state: FreeverbUiState,
    audio_stream: Option<AudioStream<FreeverbModule>>,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            ui_state: FreeverbUiState::default(),
            audio_stream: None,
        })
    }

    fn toggle_audio_stream(&mut self) {
        if self.audio_stream.is_none() {
            match AudioStream::new() {
                Ok(stream) => {
                    self.ui_state.sample_rate = stream.sample_rate();
                    stream.to_processor().push(ToProcessor::SetParameter(
                        FreeverbParameterId::Scope as usize,
                        1.0,
                    ));
                    self.ui_state.parameters.scope.value = true;
                    self.audio_stream = Some(stream);
                }
                Err(error) => println!("Failed to create audio stream: {error}"),
            }
        } else {
            self.audio_stream = None;
            self.ui_state.scope_frames.clear();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if let Some(from_processor) = self.audio_stream.as_ref().map(AudioStream::from_processor) {
            while let Some(message) = from_processor.pop() {
                match message {
                    FromFreeverb::ScopeBuffer(buffer) => {
                        self.ui_state.scope_frames.extend(buffer.iter())
                    }
                }
            }
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Freeverb").text_style(TextStyle::Heading));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut audio_enabled = self.audio_stream.is_some();
                    if ui.checkbox(&mut audio_enabled, "Enable Audio").changed() {
                        self.toggle_audio_stream();
                    }
                });
            });

            ui.separator();

            let mut to_processor = self.audio_stream.as_ref().map(AudioStream::to_processor);
            ui.add(FreeverbUi::new(&mut self.ui_state, &mut to_processor));
        });
    }
}
