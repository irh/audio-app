// #[cfg(target_os = "ios")]
mod ios;

use audio_module::{AudioModule, AudioProcessor, PopMessage, PushMessage, ToProcessor};
use cpal::{
    SupportedBufferSize,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender};
use fixed_resample::{ReadStatus, ResampleQuality, ResamplingChannelConfig, resampling_channel};
use log::{debug, error, info};
use std::num::NonZeroUsize;
use thiserror::Error;

pub const FRAMES_PER_BUFFER: usize = 1024;

#[allow(unused)]
pub struct AudioStream<M: AudioModule> {
    to_processor: Sender<ToProcessor>,
    from_processor: Receiver<<M::Processor as AudioProcessor>::OutputMessage>,
    output_stream: cpal::Stream,
    input_stream: cpal::Stream,
    sample_rate: usize,
}

impl<M: AudioModule> AudioStream<M> {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "ios")]
        {
            ios::setup_audio_session();
        }

        let channel_capacity = 1024;
        let (to_processor_sender, to_processor_receiver) =
            crossbeam_channel::bounded(channel_capacity);
        let (from_processor_sender, from_processor_receiver) =
            crossbeam_channel::bounded(channel_capacity);

        const CHANNELS: usize = 2;
        const SAMPLES_PER_BUFFER: usize = FRAMES_PER_BUFFER * CHANNELS;

        let host = cpal::default_host();

        let input_device = host
            .default_input_device()
            .ok_or(Error::DefaultDeviceUnavailable { stream: "input" })?;
        let input_device_name = input_device.name()?;
        let input_config = input_device.default_input_config()?;
        let input_channels = input_config.channels() as usize;
        let input_sample_rate = input_config.sample_rate().0;
        let input_buffer_size = match input_config.buffer_size() {
            SupportedBufferSize::Range { min, max } => (FRAMES_PER_BUFFER as u32).clamp(*min, *max),
            SupportedBufferSize::Unknown => FRAMES_PER_BUFFER as u32,
        };

        let output_device = host
            .default_output_device()
            .ok_or(Error::DefaultDeviceUnavailable { stream: "output" })?;
        let output_device_name = output_device.name()?;
        let output_config = output_device.default_output_config()?;
        let output_channels = output_config.channels() as usize;
        let output_sample_rate = output_config.sample_rate().0;
        let output_buffer_size = match output_config.buffer_size() {
            SupportedBufferSize::Range { min, max } => (FRAMES_PER_BUFFER as u32).clamp(*min, *max),
            SupportedBufferSize::Unknown => FRAMES_PER_BUFFER as u32,
        };

        let input_stream_config = cpal::StreamConfig {
            channels: input_channels as u16,
            sample_rate: cpal::SampleRate(input_sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(input_buffer_size),
        };
        debug!("Setting up input stream with config: {input_stream_config:?}");

        let mut input_buffer = [0.0f32; SAMPLES_PER_BUFFER];
        let (mut to_output, mut from_input) = resampling_channel::<f32, 2>(
            NonZeroUsize::new(2).unwrap(),
            input_sample_rate,
            output_sample_rate,
            ResamplingChannelConfig {
                quality: ResampleQuality::Low,
                ..Default::default()
            },
        );

        let mut processor = M::create_processor(input_sample_rate as usize);

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
                    for (sample, buffer_frame) in data.iter().zip(input_buffer.chunks_exact_mut(2))
                    {
                        buffer_frame.fill(*sample);
                    }

                    while let Ok(message) = to_processor_receiver.try_recv() {
                        processor.receive_message(message);
                    }

                    processor.process_buffer(&mut input_buffer, CHANNELS, |message| {
                        from_processor_sender.send(message).ok();
                    });
                    to_output.push_interleaved(&input_buffer);
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

                    while let Ok(message) = to_processor_receiver.try_recv() {
                        processor.receive_message(message);
                    }

                    processor.process_buffer(&mut input_buffer, CHANNELS, |message| {
                        from_processor_sender.send(message).ok();
                    });
                    to_output.push_interleaved(&input_buffer);
                },
                move |err| error!("Error on audio input stream: {}", err),
                None,
            ),
            _ => input_device.build_input_stream(
                &input_stream_config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    for (input_frame, buffer_frame) in data
                        .chunks_exact(input_channels)
                        .zip(input_buffer.chunks_exact_mut(2))
                    {
                        for (input_sample, buffer_sample) in
                            input_frame.iter().zip(buffer_frame.iter_mut())
                        {
                            *buffer_sample = *input_sample;
                        }
                    }

                    while let Ok(message) = to_processor_receiver.try_recv() {
                        processor.receive_message(message);
                    }

                    processor.process_buffer(&mut input_buffer, CHANNELS, |message| {
                        from_processor_sender.send(message).ok();
                    });
                    to_output.push_interleaved(&input_buffer);
                },
                move |err| error!("Error on audio input stream: {}", err),
                None,
            ),
        }?;

        let output_stream_config = cpal::StreamConfig {
            channels: output_channels as u16,
            sample_rate: cpal::SampleRate(output_sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(output_buffer_size),
        };
        debug!("Setting up output stream with config: {output_stream_config:?}");

        let mut output_buffer = [0.0f32; SAMPLES_PER_BUFFER];
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
                    match from_input.read_interleaved(&mut output_buffer) {
                        ReadStatus::Ok => {}
                        ReadStatus::InputNotReady => {}
                        ReadStatus::UnderflowOccurred { num_frames_read } => {
                            error!("input underflowed ({num_frames_read}/{})", data.len());
                        }
                        ReadStatus::OverflowCorrected {
                            num_frames_discarded,
                        } => {
                            error!("input overflowed ({num_frames_discarded} discarded)");
                        }
                    }

                    for (output_sample, processed_frame) in
                        data.iter_mut().zip(output_buffer.chunks_exact(CHANNELS))
                    {
                        *output_sample = processed_frame.iter().copied().sum();
                    }
                },
                move |err| error!("Error on audio output stream: {}", err),
                None,
            ),
            2 => output_device.build_output_stream(
                &output_stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| match from_input
                    .read_interleaved(data)
                {
                    ReadStatus::Ok => {}
                    ReadStatus::InputNotReady => {}
                    ReadStatus::UnderflowOccurred { num_frames_read } => {
                        error!("input underflowed ({num_frames_read}/{})", data.len() / 2);
                    }
                    ReadStatus::OverflowCorrected {
                        num_frames_discarded,
                    } => {
                        error!("input overflowed ({num_frames_discarded} discarded)");
                    }
                },
                move |err| error!("Error on audio output stream: {}", err),
                None,
            ),
            _ => output_device.build_output_stream(
                &output_stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    match from_input.read_interleaved(&mut output_buffer) {
                        ReadStatus::Ok => {}
                        ReadStatus::InputNotReady => {}
                        ReadStatus::UnderflowOccurred { num_frames_read } => {
                            error!(
                                "input underflowed ({num_frames_read}/{})",
                                data.len() / output_channels
                            );
                        }
                        ReadStatus::OverflowCorrected {
                            num_frames_discarded,
                        } => {
                            error!("input overflowed ({num_frames_discarded} discarded)");
                        }
                    }

                    for (output_frame, processed_frame) in data
                        .chunks_exact_mut(output_channels)
                        .zip(output_buffer.chunks_exact(CHANNELS))
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

    pub fn to_processor(&self) -> ProcessorSender {
        ProcessorSender(self.to_processor.clone())
    }

    pub fn from_processor(&self) -> ProcessorReceiver<M::Processor> {
        ProcessorReceiver(self.from_processor.clone())
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }
}

/// An implementation of [PushMessage] that wraps a crossbeam_channel sender.
#[derive(Clone)]
pub struct ProcessorSender(Sender<ToProcessor>);

impl ProcessorSender {
    pub fn new(sender: Sender<ToProcessor>) -> Self {
        Self(sender)
    }
}

impl PushMessage for ProcessorSender {
    fn push(&self, command: ToProcessor) {
        if self.0.send(command).is_err() {
            error!("Channel is disconnected");
        }
    }
}

/// An implementation of [PopMessage] that wraps a crossbeam_channel receiver.
#[derive(Clone)]
pub struct ProcessorReceiver<P: AudioProcessor>(Receiver<P::OutputMessage>);

impl<P: AudioProcessor> ProcessorReceiver<P> {
    pub fn new(receiver: Receiver<P::OutputMessage>) -> Self {
        Self(receiver)
    }
}

impl<P: AudioProcessor> PopMessage<P::OutputMessage> for ProcessorReceiver<P> {
    fn pop(&self) -> Option<P::OutputMessage> {
        self.0.try_recv().ok()
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
