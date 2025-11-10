#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use crate::DEFAULT_BUFFER_SIZE;
use audio_module::{AudioModule, AudioProcessor, PopMessage, PushMessage, ToProcessor};
use cpal::{
    SupportedBufferSize,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender};
use fixed_resample::{
    PushStatus, ReadStatus, ResampleQuality, ResamplingChannelConfig, resampling_channel,
};
use log::{debug, error, info};
use std::num::NonZeroUsize;
use thiserror::Error;

#[allow(unused)]
pub struct AudioStream<M: AudioModule> {
    to_processor: ToProcessorSender,
    from_processor: FromProcessorReceiver<M::Processor>,
    input_stream: cpal::Stream,
    output_stream: cpal::Stream,
    sample_rate: usize,
}

impl<M: AudioModule> AudioStream<M> {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "android")]
        {
            android::request_recording_permission();
        }

        #[cfg(target_os = "ios")]
        {
            ios::setup_audio_session();
        }

        let channel_capacity = 1024;

        let (sender, receiver) = crossbeam_channel::bounded(channel_capacity);
        let to_processor_sender = ToProcessorSender(sender);
        let to_processor_receiver = ToProcessorReceiver(receiver);

        let (sender, receiver) = crossbeam_channel::bounded(channel_capacity);
        let from_processor_sender = FromProcessorSender::<M::Processor>(sender);
        let from_processor_receiver = FromProcessorReceiver(receiver);

        let channels = 2;

        let host = cpal::default_host();

        let input_device = host
            .default_input_device()
            .ok_or(Error::DefaultDeviceUnavailable { stream: "input" })?;
        let input_device_name = input_device.name()?;
        let input_config = input_device.default_input_config()?;
        debug!("default input config: {input_config:?}");
        let input_channels = input_config.channels() as usize;
        let input_sample_rate = input_config.sample_rate().0;
        let input_buffer_size = match input_config.buffer_size() {
            SupportedBufferSize::Range { min, max } => {
                (DEFAULT_BUFFER_SIZE as u32).clamp(*min, *max)
            }
            SupportedBufferSize::Unknown => DEFAULT_BUFFER_SIZE as u32,
        };
        let mut input_buffer = vec![0.0; input_buffer_size as usize * channels];

        let output_device = host
            .default_output_device()
            .ok_or(Error::DefaultDeviceUnavailable { stream: "output" })?;
        let output_device_name = output_device.name()?;
        let output_config = output_device.default_output_config()?;
        debug!("default output config: {output_config:?}");
        let output_channels = output_config.channels() as usize;
        let output_sample_rate = output_config.sample_rate().0;
        let output_buffer_size = match output_config.buffer_size() {
            SupportedBufferSize::Range { min, max } => {
                (DEFAULT_BUFFER_SIZE as u32).clamp(*min, *max)
            }
            SupportedBufferSize::Unknown => DEFAULT_BUFFER_SIZE as u32,
        };
        let mut output_buffer = vec![0.0; output_buffer_size as usize * channels];

        let input_buffer_duration = (input_buffer_size as f64) / (input_sample_rate as f64);
        let output_buffer_duration = (output_buffer_size as f64) / (output_sample_rate as f64);
        debug!("input buffer duration: {input_buffer_duration}, output: {output_buffer_duration}");

        // Set up the input -> output channel
        let latency_seconds = input_buffer_duration.max(output_buffer_duration);
        let capacity_seconds = latency_seconds * 2.0;
        let (mut to_output, mut from_input) = resampling_channel::<f32, 2>(
            NonZeroUsize::new(channels).unwrap(),
            input_sample_rate,
            output_sample_rate,
            ResamplingChannelConfig {
                latency_seconds,
                capacity_seconds,
                quality: ResampleQuality::Low,
                ..Default::default()
            },
        );

        let mut processor = M::create_processor(input_sample_rate as usize);

        let mut process_fn = move |buffer: &mut [f32]| {
            if !to_output.output_stream_ready() {
                return;
            }

            processor.process_buffer(
                buffer,
                channels,
                &to_processor_receiver,
                &from_processor_sender,
            );

            match to_output.push_interleaved(&buffer) {
                PushStatus::Ok => {}
                PushStatus::OverflowOccurred { num_frames_pushed } => {
                    error!("input -> output push overflow ({num_frames_pushed})");
                }
                PushStatus::UnderflowCorrected {
                    num_zero_frames_pushed,
                } => {
                    error!("input -> output push underflow ({num_zero_frames_pushed})");
                }
                PushStatus::OutputNotReady => {
                    // We exited early above when output isn't ready
                    unreachable!();
                }
            }
        };

        let input_stream_config = cpal::StreamConfig {
            channels: input_channels as u16,
            sample_rate: cpal::SampleRate(input_sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(input_buffer_size),
        };
        debug!("Setting up input stream with config: {input_stream_config:?}");

        let input_stream = match input_channels {
            0 => {
                return Err(Error::DeviceHasNoAvailableChannels {
                    device_name: input_device_name,
                    stream: "input",
                });
            }
            1 => input_device.build_input_stream(
                &input_stream_config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    for (sample, buffer_frame) in
                        data.iter().zip(input_buffer.chunks_exact_mut(channels))
                    {
                        buffer_frame.fill(*sample);
                    }

                    process_fn(&mut input_buffer);
                },
                move |err| error!("Error on audio input stream: {}", err),
                None,
            ),
            2 => input_device.build_input_stream(
                &input_stream_config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    for (input_sample, buffer_sample) in data.iter().zip(input_buffer.iter_mut()) {
                        *buffer_sample = *input_sample;
                    }

                    process_fn(&mut input_buffer);
                },
                move |err| error!("Error on audio input stream: {}", err),
                None,
            ),
            _ => input_device.build_input_stream(
                &input_stream_config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    for (input_frame, buffer_frame) in data
                        .chunks_exact(input_channels)
                        .zip(input_buffer.chunks_exact_mut(channels))
                    {
                        for (input_sample, buffer_sample) in
                            input_frame.iter().zip(buffer_frame.iter_mut())
                        {
                            *buffer_sample = *input_sample;
                        }
                    }

                    process_fn(&mut input_buffer);
                },
                move |err| error!("Error on audio input stream: {}", err),
                None,
            ),
        }?;

        let mut receive_frame_fn =
            move |buffer: &mut [f32]| match from_input.read_interleaved(buffer) {
                ReadStatus::Ok => {}
                ReadStatus::InputNotReady => {}
                ReadStatus::UnderflowOccurred { num_frames_read } => {
                    error!(
                        "input -> output read underflowed ({num_frames_read}/{})",
                        buffer.len() / channels
                    );
                }
                ReadStatus::OverflowCorrected {
                    num_frames_discarded,
                } => {
                    error!("input -> output read overflowed ({num_frames_discarded} discarded)");
                }
            };

        let output_stream_config = cpal::StreamConfig {
            channels: output_channels as u16,
            sample_rate: cpal::SampleRate(output_sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(output_buffer_size),
        };
        debug!("Setting up output stream with config: {output_stream_config:?}");

        let output_stream = match output_channels {
            0 => {
                return Err(Error::DeviceHasNoAvailableChannels {
                    device_name: output_device_name,
                    stream: "output",
                });
            }
            1 => output_device.build_output_stream(
                &output_stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    receive_frame_fn(&mut output_buffer);

                    for (output_sample, processed_frame) in
                        data.iter_mut().zip(output_buffer.chunks_exact(channels))
                    {
                        *output_sample = processed_frame.iter().copied().sum();
                    }
                },
                move |err| error!("Error on audio output stream: {}", err),
                None,
            ),
            2 => output_device.build_output_stream(
                &output_stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    receive_frame_fn(data);
                },
                move |err| error!("Error on audio output stream: {}", err),
                None,
            ),
            _ => output_device.build_output_stream(
                &output_stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    receive_frame_fn(&mut output_buffer);

                    for (output_frame, processed_frame) in data
                        .chunks_exact_mut(output_channels)
                        .zip(output_buffer.chunks_exact(channels))
                    {
                        for (output_sample, processed_sample) in
                            output_frame.iter_mut().zip(processed_frame.iter())
                        {
                            *output_sample = *processed_sample;
                        }
                    }
                },
                move |err| error!("Error on audio output stream: {}", err),
                None,
            ),
        }?;

        input_stream.play()?;
        output_stream.play()?;

        info!(
            "\
Audio stream started:
  input - channels: {input_channels}, sample rate: {input_sample_rate}
  output - channels: {output_channels}, sample rate: {output_sample_rate}"
        );

        Ok(AudioStream {
            to_processor: to_processor_sender,
            from_processor: from_processor_receiver,
            input_stream,
            output_stream,
            sample_rate: input_sample_rate as usize,
        })
    }

    pub fn to_processor(&self) -> ToProcessorSender {
        self.to_processor.clone()
    }

    pub fn from_processor(&self) -> FromProcessorReceiver<M::Processor> {
        self.from_processor.clone()
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }
}

/// An implementation of [PushMessage] that sends messages to [ToProcessorReceiver].
#[derive(Clone)]
pub struct ToProcessorSender(Sender<ToProcessor>);

impl ToProcessorSender {
    pub fn new(sender: Sender<ToProcessor>) -> Self {
        Self(sender)
    }
}

impl PushMessage<ToProcessor> for ToProcessorSender {
    fn push(&self, message: ToProcessor) -> bool {
        self.0.try_send(message).is_ok()
    }
}

impl PartialEq for ToProcessorSender {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_channel(&other.0)
    }
}

/// An implementation of [PopMessage] that gets passed into the processor.
///
/// Receives messages from [ToProcessorSender].
#[derive(Clone)]
pub struct ToProcessorReceiver(Receiver<ToProcessor>);

impl ToProcessorReceiver {
    pub fn new(receiver: Receiver<ToProcessor>) -> Self {
        Self(receiver)
    }
}

impl PopMessage<ToProcessor> for ToProcessorReceiver {
    fn pop(&self) -> Option<ToProcessor> {
        self.0.try_recv().ok()
    }
}

/// An implementation of [PushMessage] that sends messages to [FromProcessorReceiver].
#[derive(Clone)]
pub struct FromProcessorSender<P: AudioProcessor>(Sender<P::OutputMessage>);

impl<P: AudioProcessor> FromProcessorSender<P> {
    pub fn new(sender: Sender<P::OutputMessage>) -> Self {
        Self(sender)
    }
}

impl<P: AudioProcessor> PushMessage<P::OutputMessage> for FromProcessorSender<P> {
    fn push(&self, message: P::OutputMessage) -> bool {
        self.0.try_send(message).is_ok()
    }
}

/// An implementation of [PopMessage] that wraps a crossbeam_channel receiver.
pub struct FromProcessorReceiver<P: AudioProcessor>(Receiver<P::OutputMessage>);

impl<P: AudioProcessor> FromProcessorReceiver<P> {
    pub fn new(receiver: Receiver<P::OutputMessage>) -> Self {
        Self(receiver)
    }
}

impl<P: AudioProcessor> PopMessage<P::OutputMessage> for FromProcessorReceiver<P> {
    fn pop(&self) -> Option<P::OutputMessage> {
        self.0.try_recv().ok()
    }
}

impl<P: AudioProcessor> Clone for FromProcessorReceiver<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("default {stream} audio device is unavailable")]
    DefaultDeviceUnavailable { stream: &'static str },
    #[error("no channels available for the {stream} stream (device: {device_name})")]
    DeviceHasNoAvailableChannels {
        device_name: String,
        stream: &'static str,
    },
    #[error("underrun, missing {missing_frames} frames")]
    FramesMissing { missing_frames: usize },
    #[error("invalid stream buffer size (size: {buffer_size}, max: {max_size})")]
    InvalidStreamBufferSize { buffer_size: u32, max_size: u32 },

    #[error(transparent)]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    DefaultStreamConfig(#[from] cpal::DefaultStreamConfigError),
    #[error(transparent)]
    DevicesError(#[from] cpal::DevicesError),
    #[error(transparent)]
    DeviceNameError(#[from] cpal::DeviceNameError),
    #[error(transparent)]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error(transparent)]
    PauseStream(#[from] cpal::PauseStreamError),
    #[error(transparent)]
    Stream(#[from] cpal::StreamError),
}

pub type Result<T> = std::result::Result<T, Error>;
