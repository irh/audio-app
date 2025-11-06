use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::widgets::{PhaseScope, freeverb_parameters};
use audio_module::{PopMessage, PushMessage, ToProcessor};
use audio_stream::AudioStream;
use freeverb_module::{FreeverbModule, FreeverbParameterId, FreeverbParameters, FromFreeverb};
use iced::{
    Element, Subscription,
    alignment::Vertical,
    stream,
    widget::{checkbox, column, horizontal_rule, horizontal_space, row, text, vertical_rule},
};

pub type ScopeFrames = Arc<Mutex<VecDeque<(f32, f32)>>>;

pub struct App {
    parameters: FreeverbParameters,
    audio_stream: Option<AudioStream<FreeverbModule>>,
    sample_rate: usize,
    scope_frames: ScopeFrames,
}

impl App {
    pub fn new(parameters: FreeverbParameters) -> Self {
        Self {
            parameters,
            audio_stream: None,
            sample_rate: 0,
            scope_frames: Arc::new(Mutex::new(VecDeque::with_capacity(1024))),
        }
    }
}

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        column![
            row![
                text("Freeverb").size(40),
                horizontal_space(),
                checkbox("Audio Enabled", self.audio_stream.is_some())
                    .on_toggle(Message::SetEnabled),
            ]
            .align_y(Vertical::Center),
            horizontal_rule(1),
            row![
                freeverb_parameters(&self.parameters),
                vertical_rule(1),
                PhaseScope::new(self.audio_stream.is_some(), self.scope_frames.clone()),
            ]
            .spacing(10)
        ]
        .padding(20)
        .spacing(10)
        .into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SetEnabled(enabled) => {
                if enabled {
                    match AudioStream::new() {
                        Ok(stream) => {
                            self.sample_rate = stream.sample_rate();
                            stream.to_processor().push(ToProcessor::SetParameter(
                                FreeverbParameterId::Scope as usize,
                                1.0,
                            ));
                            self.parameters.scope.value = true;
                            self.audio_stream = Some(stream);
                        }
                        Err(error) => println!("Failed to create audio stream: {error}"),
                    }
                } else {
                    self.audio_stream = None;
                    self.scope_frames.lock().unwrap().clear();
                }
            }
            Message::SetFloat(id, value) => {
                match id {
                    FreeverbParameterId::Dampening => self.parameters.dampening.value = value,
                    FreeverbParameterId::Width => self.parameters.width.value = value,
                    FreeverbParameterId::RoomSize => self.parameters.room_size.value = value,
                    FreeverbParameterId::Dry => self.parameters.dry.value = value,
                    FreeverbParameterId::Wet => self.parameters.wet.value = value,
                    _ => unreachable!(),
                }

                if let Some(stream) = &self.audio_stream {
                    stream
                        .to_processor()
                        .push(ToProcessor::SetParameter(id as usize, value));
                }
            }
            Message::SetBool(id, value) => {
                match id {
                    FreeverbParameterId::Freeze => self.parameters.freeze.value = value,
                    _ => unreachable!(),
                }

                if let Some(stream) = &self.audio_stream {
                    stream.to_processor().push(ToProcessor::SetParameter(
                        id as usize,
                        if value { 1.0 } else { 0.0 },
                    ));
                }
            }
            Message::FromProcessor(message) => match message {
                FromFreeverb::ScopeBuffer(buffer) => {
                    let mut frames = self.scope_frames.lock().unwrap();
                    frames.extend(buffer.iter());

                    let scope_tail_duration = 0.2;
                    let scope_frame_count =
                        (scope_tail_duration * self.sample_rate as f64) as usize;
                    let frames_to_drop = frames.len().saturating_sub(scope_frame_count);
                    // `VecDeque::truncate_front` can be used here once it's stable.
                    frames.drain(0..frames_to_drop);
                }
            },
        }
    }

    /// Sets up a subscription to receive messages coming from the processor
    pub fn subscription(&self) -> Subscription<Message> {
        let Some(audio_stream) = &self.audio_stream else {
            return Subscription::none();
        };

        let message_receiver = audio_stream.from_processor();
        Subscription::run_with_id(
            0,
            stream::channel(128, async move |mut sender| {
                let update_interval = Duration::from_millis(10);
                loop {
                    while let Some(message) = message_receiver.pop() {
                        sender.try_send(Message::FromProcessor(message)).ok();
                    }

                    tokio::time::sleep(update_interval).await;
                }
            }),
        )
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    SetEnabled(bool),
    SetFloat(FreeverbParameterId, f32),
    SetBool(FreeverbParameterId, bool),
    FromProcessor(FromFreeverb),
}
