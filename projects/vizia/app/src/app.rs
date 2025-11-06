use crate::widgets::{PhaseScope, ScopeFrames, parameter_slider, parameter_toggle};
use audio_module::{PopMessage, PushMessage, ToProcessor};
use audio_stream::AudioStream;
use freeverb_module::{FreeverbModule, FreeverbParameterId, FreeverbParameters, FromFreeverb};
use vizia::prelude::*;

pub const UI_SIZE: (u32, u32) = (700, 450);

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    EnabledChanged,
    SetFloat(FreeverbParameterId, f32),
    ToggleBool(FreeverbParameterId),
    FromProcessor(FromFreeverb),
}

#[derive(Lens)]
pub struct App {
    parameters: FreeverbParameters,
    audio_stream: Option<AudioStream<FreeverbModule>>,
    update_timer: Option<Timer>,
    sample_rate: usize,
    scope_frames: ScopeFrames,
}

impl App {
    pub fn build(cx: &mut Context, parameters: FreeverbParameters) {
        Self {
            parameters,
            audio_stream: None,
            sample_rate: 0,
            scope_frames: ScopeFrames::with_capacity(1024),
            update_timer: None,
        }
        .build(cx);

        let audio_enabled = Self::audio_stream.map(|stream| stream.is_some());

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "Freeverb").font_size(40.0);

                ToggleButton::new(cx, audio_enabled, {
                    move |cx| Label::new(cx, "Audio Enabled")
                })
                .on_toggle(move |cx| cx.emit(Message::EnabledChanged));
            })
            .horizontal_gap(Stretch(1.0))
            .height(Auto);

            Divider::horizontal(cx);

            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    parameter_slider(cx, Self::parameters.map_ref(|p| &p.dampening));
                    parameter_slider(cx, Self::parameters.map_ref(|p| &p.width));
                    parameter_slider(cx, Self::parameters.map_ref(|p| &p.room_size));
                    parameter_toggle(cx, Self::parameters.map_ref(|p| &p.freeze));
                    parameter_slider(cx, Self::parameters.map_ref(|p| &p.dry));
                    parameter_slider(cx, Self::parameters.map_ref(|p| &p.wet));
                })
                .width(Pixels(300.0))
                .vertical_gap(Pixels(20.0));

                Divider::vertical(cx);

                PhaseScope::new(cx, audio_enabled, Self::scope_frames).min_size(Pixels(200.0));
            })
            .horizontal_gap(Pixels(10.0));
        })
        .size(Stretch(1.0))
        .padding(Pixels(20.0))
        .vertical_gap(Pixels(10.0));
    }
}

impl Model for App {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|message, _meta| match message {
            Message::EnabledChanged => {
                if self.audio_stream.is_none() {
                    match AudioStream::new() {
                        Ok(stream) => {
                            self.sample_rate = stream.sample_rate();
                            stream.to_processor().push(ToProcessor::SetParameter(
                                FreeverbParameterId::Scope as usize,
                                1.0,
                            ));
                            self.parameters.scope.value = true;
                            let from_processor = stream.from_processor();
                            self.audio_stream = Some(stream);

                            println!("Starting timer");
                            let update_timer = cx.add_timer(
                                Duration::from_millis(10),
                                None,
                                move |cx, _reason| {
                                    while let Some(message) = from_processor.pop() {
                                        cx.emit(Message::FromProcessor(message));
                                    }
                                },
                            );
                            cx.start_timer(update_timer);
                            self.update_timer = Some(update_timer);
                        }
                        Err(error) => println!("Failed to create audio stream: {error}"),
                    }
                } else {
                    if let Some(timer) = self.update_timer.take() {
                        cx.stop_timer(timer);
                    }
                    self.audio_stream = None;
                    self.scope_frames.clear();
                }
            }
            Message::SetFloat(id, value) => {
                match id {
                    FreeverbParameterId::Dampening => self.parameters.dampening.value = *value,
                    FreeverbParameterId::Width => self.parameters.width.value = *value,
                    FreeverbParameterId::RoomSize => self.parameters.room_size.value = *value,
                    FreeverbParameterId::Dry => self.parameters.dry.value = *value,
                    FreeverbParameterId::Wet => self.parameters.wet.value = *value,
                    _ => unreachable!(),
                }

                if let Some(stream) = &self.audio_stream {
                    stream
                        .to_processor()
                        .push(ToProcessor::SetParameter(*id as usize, *value));
                }
            }
            Message::ToggleBool(id) => {
                let value = match id {
                    FreeverbParameterId::Freeze => &mut self.parameters.freeze.value,
                    _ => unreachable!(),
                };

                *value = !*value;

                if let Some(stream) = &self.audio_stream {
                    stream.to_processor().push(ToProcessor::SetParameter(
                        *id as usize,
                        if *value { 1.0 } else { 0.0 },
                    ));
                }
            }
            Message::FromProcessor(message) => match message {
                FromFreeverb::ScopeBuffer(buffer) => {
                    self.scope_frames.extend(buffer.iter());

                    let scope_tail_duration = 0.2;
                    let scope_frame_count =
                        (scope_tail_duration * self.sample_rate as f64) as usize;
                    let frames_to_drop = self.scope_frames.len().saturating_sub(scope_frame_count);
                    // `VecDeque::truncate_front` can be used here once it's stable.
                    self.scope_frames.drain(0..frames_to_drop);
                }
            },
        });
    }
}
