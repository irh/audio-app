#[cfg(target_arch = "wasm32")]
mod wasm;

use audio_module::{
    AudioModule, AudioProcessor, BoolParameter, FloatParameter, Parameters, PercentStringConverter,
    PopMessage, PushMessage, ToProcessor,
};
use audio_stream::FRAMES_PER_BUFFER;
use freeverb::{Float, Freeverb};

pub struct FreeverbModule;

impl AudioModule for FreeverbModule {
    const NAME: &'static str = "Freeverb";

    type Parameters = FreeverbParameters;
    type Processor = FreeverbProcessor;

    fn create_processor(sample_rate: usize) -> Self::Processor {
        FreeverbProcessor::new(sample_rate)
    }
}

pub enum ToFreeverb {
    SetScopeEnabled(bool),
}

#[derive(Clone, Debug)]
#[cfg_attr(target_arch = "wasm32", derive(serde::Serialize, serde::Deserialize))]
pub enum FromFreeverb {
    #[cfg_attr(target_arch = "wasm32", serde(with = "serde_arrays"))]
    ScopeBuffer([(f32, f32); FRAMES_PER_BUFFER]),
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum FreeverbParameterId {
    Dampening,
    Width,
    RoomSize,
    Freeze,
    Dry,
    Wet,
    Scope,
}

impl FreeverbParameterId {
    pub const fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl TryFrom<usize> for FreeverbParameterId {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Dampening),
            1 => Ok(Self::Width),
            2 => Ok(Self::RoomSize),
            3 => Ok(Self::Freeze),
            4 => Ok(Self::Dry),
            5 => Ok(Self::Wet),
            6 => Ok(Self::Scope),
            _ => Err(()),
        }
    }
}

#[derive(Clone)]
pub struct FreeverbParameters {
    pub dampening: FloatParameter,
    pub width: FloatParameter,
    pub room_size: FloatParameter,
    pub freeze: BoolParameter,
    pub dry: FloatParameter,
    pub wet: FloatParameter,
    pub scope: BoolParameter,
}

impl Default for FreeverbParameters {
    fn default() -> Self {
        Self {
            dampening: FloatParameter::builder(
                "Dampening",
                FreeverbParameterId::Dampening as usize,
            )
            .string_converter(percent_string_converter())
            .default_user_value(0.75)
            .build(),
            width: FloatParameter::builder("Width", FreeverbParameterId::Width as usize)
                .string_converter(percent_string_converter())
                .default_user_value(0.5)
                .build(),
            room_size: FloatParameter::builder("Room Size", FreeverbParameterId::RoomSize as usize)
                .string_converter(percent_string_converter())
                .default_user_value(0.25)
                .build(),
            freeze: BoolParameter::new("Freeze", FreeverbParameterId::Freeze as usize, false),
            dry: FloatParameter::builder("Dry", FreeverbParameterId::Dry as usize)
                .string_converter(percent_string_converter())
                .default_user_value(0.0)
                .build(),
            wet: FloatParameter::builder("Wet", FreeverbParameterId::Wet as usize)
                .string_converter(percent_string_converter())
                .default_user_value(0.33)
                .build(),
            scope: BoolParameter::new("Scope", FreeverbParameterId::Scope as usize, false),
        }
    }
}

fn percent_string_converter() -> PercentStringConverter {
    PercentStringConverter::default()
}

impl Parameters for FreeverbParameters {}

pub struct FreeverbProcessor<T: Float = f64> {
    freeverb: Freeverb<T>,
    scope_enabled: bool,
}

impl<T: Float> FreeverbProcessor<T> {
    pub fn new(sample_rate: usize) -> Self {
        Self {
            freeverb: Freeverb::new(sample_rate),
            scope_enabled: false,
        }
    }

    fn receive_message(&mut self, message: ToProcessor) {
        match message {
            ToProcessor::SetParameter(id, value) => {
                let Ok(parameter_id) = FreeverbParameterId::try_from(id) else {
                    println!("Invalid parameter ID: {id}"); // TODO: Return an error
                    return;
                };

                match parameter_id {
                    FreeverbParameterId::Dampening => {
                        self.freeverb.set_dampening(value.into());
                    }
                    FreeverbParameterId::Width => {
                        self.freeverb.set_width(value.into());
                    }
                    FreeverbParameterId::RoomSize => {
                        self.freeverb.set_room_size(value.into());
                    }
                    FreeverbParameterId::Freeze => {
                        self.freeverb.set_freeze(value != 0.0);
                    }
                    FreeverbParameterId::Dry => {
                        self.freeverb.set_dry(value.into());
                    }
                    FreeverbParameterId::Wet => {
                        self.freeverb.set_wet(value.into());
                    }
                    FreeverbParameterId::Scope => {
                        self.scope_enabled = value != 0.0;
                    }
                }
            }
            ToProcessor::BeginEdit(_) => {}
            ToProcessor::EndEdit(_) => {}
        }
    }
}

impl<T: Float> AudioProcessor for FreeverbProcessor<T> {
    type OutputMessage = FromFreeverb;

    fn process_buffer<To, From>(
        &mut self,
        buffer: &mut [f32],
        channels: usize,
        to_processor: &To,
        from_processor: &From,
    ) where
        To: PopMessage<ToProcessor>,
        From: PushMessage<Self::OutputMessage>,
    {
        debug_assert_eq!(channels, 2);
        let (frames, remainder) = buffer.as_chunks_mut::<2>();
        debug_assert_eq!(remainder.len(), 0);

        while let Some(message) = to_processor.pop() {
            self.receive_message(message);
        }

        if self.scope_enabled {
            let mut scope_buffer = [(0.0, 0.0); FRAMES_PER_BUFFER];

            for (process_frame, scope_frame) in frames.iter_mut().zip(scope_buffer.iter_mut()) {
                let (out_left, out_right) = self
                    .freeverb
                    .tick((T::from(process_frame[0]), T::from(process_frame[1])));
                process_frame[0] = out_left.to_f32();
                process_frame[1] = out_right.to_f32();
                *scope_frame = (process_frame[0], process_frame[1]);
            }

            from_processor.push(FromFreeverb::ScopeBuffer(scope_buffer));
        } else {
            for frame in frames.iter_mut() {
                let (out_left, out_right) =
                    self.freeverb.tick((T::from(frame[0]), T::from(frame[1])));
                frame[0] = out_left.to_f32();
                frame[1] = out_right.to_f32();
            }
        }
    }
}
