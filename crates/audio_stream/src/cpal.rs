#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use crate::FRAMES_PER_UPDATE;
use audio_module::{AudioModule, AudioProcessor, PopMessage, PushMessage, ToProcessor};
use audio_thread_priority::promote_current_thread_to_real_time;
use cpal::{
    BufferSize, Device, InputCallbackInfo, OutputCallbackInfo, SampleRate, Stream, StreamConfig,
    SupportedBufferSize, SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use fixed_resample::{
    PushStatus, ReadStatus, ResampleQuality, ResamplingChannelConfig, ResamplingCons,
    ResamplingProd, resampling_channel,
};
use log::{debug, error, info, warn};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle, sleep},
    time::{Duration, Instant},
};
use thiserror::Error;

const CHANNELS: usize = 2;

pub struct AudioStream<M: AudioModule> {
    to_processor: ToProcessorSender,
    from_processor: FromProcessorReceiver<M::Processor>,
    sample_rate: usize,
    exit_flag: Arc<AtomicBool>,
    processor_thread: Option<JoinHandle<()>>,
    stream_manager_thread: Option<JoinHandle<()>>,
}

impl<M: AudioModule> AudioStream<M> {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "android")]
        {
            if let Err(error) = android::request_recording_permission() {
                error!("Failed to get recording permission: {error}");
            }
        }

        #[cfg(target_os = "ios")]
        {
            ios::setup_audio_session();
        }

        let channel_capacity = 1024;
        let processor_sample_rate = 44100;

        let (to_processor_sender, to_processor_receiver) = bounded(channel_capacity);
        let to_processor_sender = ToProcessorSender(to_processor_sender);
        let to_processor_receiver = ToProcessorReceiver(to_processor_receiver);

        let (from_processor_sender, from_processor_receiver) = bounded(channel_capacity);
        let from_processor_sender = FromProcessorSender::<M::Processor>(from_processor_sender);
        let from_processor_receiver =
            FromProcessorReceiver::<M::Processor>(from_processor_receiver);

        let (stream_error_sender, stream_error_receiver) = bounded(channel_capacity);

        let (stream_channels_sender, stream_channels_receiver) = unbounded();
        let exit_flag = Arc::new(AtomicBool::new(false));

        // Start the stream manager thread
        let stream_manager_thread = {
            thread::Builder::new()
                .name("stream_manager".to_string())
                .spawn({
                    let exit_flag = exit_flag.clone();
                    let stream_error_sender = stream_error_sender.clone();

                    move || {
                        stream_manager_thread(
                            processor_sample_rate,
                            stream_channels_sender,
                            stream_error_sender,
                            stream_error_receiver,
                            exit_flag,
                        );
                    }
                })?
        };

        // Start the processor thread
        let processor_thread = {
            thread::Builder::new()
                .name("audio_processor".to_string())
                .spawn({
                    let exit_flag = exit_flag.clone();
                    move || {
                        processor_thread::<M>(
                            processor_sample_rate,
                            to_processor_receiver,
                            from_processor_sender,
                            stream_channels_receiver,
                            stream_error_sender,
                            exit_flag,
                        );
                    }
                })?
        };

        Ok(AudioStream {
            to_processor: to_processor_sender,
            from_processor: from_processor_receiver,
            sample_rate: processor_sample_rate,
            exit_flag,
            processor_thread: Some(processor_thread),
            stream_manager_thread: Some(stream_manager_thread),
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

impl<M: AudioModule> Drop for AudioStream<M> {
    fn drop(&mut self) {
        self.exit_flag.store(true, Ordering::Relaxed);

        self.processor_thread.take().map(JoinHandle::join);
        self.stream_manager_thread.take().map(JoinHandle::join);
    }
}

struct Streams {
    input: Stream,
    input_config: SupportedStreamConfig,

    output: Stream,
    output_config: SupportedStreamConfig,
}

impl Streams {
    fn config_change_detected(&self) -> bool {
        let host = cpal::default_host();

        let new_input_config = host
            .default_input_device()
            .and_then(|device| device.default_input_config().ok());
        let new_output_config = host
            .default_output_device()
            .and_then(|device| device.default_output_config().ok());

        new_input_config.is_none_or(|config| config != self.input_config)
            || new_output_config.is_none_or(|config| config != self.output_config)
    }
}

struct StreamChannels {
    from_input: ResamplingCons<f32>,
    to_output: ResamplingProd<f32, CHANNELS>,
}

fn stream_manager_thread(
    processor_sample_rate: usize,
    stream_channels_sender: Sender<Option<StreamChannels>>,
    stream_error_sender: Sender<Error>,
    stream_error_receiver: Receiver<Error>,
    exit_flag: Arc<AtomicBool>,
) {
    let update_interval = Duration::from_millis(50);
    let config_check_interval = Duration::from_secs(1);
    let mut streams: Option<Streams> = None;

    let mut next_config_check = Instant::now();

    while !exit_flag.load(Ordering::Relaxed) {
        while let Ok(error) = stream_error_receiver.try_recv() {
            error!("{error}");
        }

        let now = Instant::now();
        if now > next_config_check {
            next_config_check = now + config_check_interval;

            if streams.as_ref().is_none_or(Streams::config_change_detected) {
                debug!("Stream config change detected");

                if stream_channels_sender.send(None).is_err() {
                    warn!("Failed to send channels reset to processor thread, exiting");
                    break;
                }

                if let Some(streams) = streams.take() {
                    if let Err(error) = streams.input.pause() {
                        warn!("Failed to pause input stream: {error}");
                    }
                    if let Err(error) = streams.output.pause() {
                        warn!("Failed to pause output stream: {error}");
                    }
                }

                match initialize_streams(processor_sample_rate, stream_error_sender.clone()) {
                    Ok((new_streams, stream_channels)) => {
                        streams = Some(new_streams);
                        if stream_channels_sender.send(Some(stream_channels)).is_err() {
                            warn!("Failed to send stream channels to processor thread, exiting");
                            break;
                        }
                    }
                    Err(error) => {
                        error!("Failed to initialize streams: {error}");
                    }
                }
            }
        }

        sleep(update_interval);
    }

    info!("Stream manager thread exiting");
}

fn processor_thread<M: AudioModule>(
    sample_rate: usize,
    to_processor: ToProcessorReceiver,
    from_processor: FromProcessorSender<M::Processor>,
    from_monitor_thread: Receiver<Option<StreamChannels>>,
    stream_error_sender: Sender<Error>,
    exit_flag: Arc<AtomicBool>,
) {
    if let Err(error) =
        promote_current_thread_to_real_time(FRAMES_PER_UPDATE as u32, sample_rate as u32)
    {
        error!("Failed to promote processor thread priority: {error}");
    }

    let mut processor = M::create_processor(sample_rate);
    let mut stream_channels = None;

    let frames_per_update = FRAMES_PER_UPDATE;
    let mut buffer = vec![0.0f32; frames_per_update * CHANNELS];

    let mut update_time = Instant::now();
    let update_interval = Duration::from_secs_f64(frames_per_update as f64 / sample_rate as f64);

    while !exit_flag.load(Ordering::Relaxed) {
        while let Ok(new_channels) = from_monitor_thread.try_recv() {
            stream_channels = new_channels;
        }

        if let Some(channels) = &mut stream_channels {
            let input_received = match channels.from_input.read_interleaved(&mut buffer) {
                ReadStatus::Ok => true,
                ReadStatus::InputNotReady => false,
                ReadStatus::UnderflowOccurred { num_frames_read } => {
                    stream_error_sender
                        .try_send(Error::StreamReadUnderflow {
                            name: "input -> processor",
                            frames: num_frames_read,
                        })
                        .ok();
                    true
                }
                ReadStatus::OverflowCorrected {
                    num_frames_discarded,
                } => {
                    stream_error_sender
                        .try_send(Error::StreamReadOverflow {
                            name: "input -> processor",
                            frames: num_frames_discarded,
                        })
                        .ok();
                    true
                }
            };

            if input_received {
                processor.process_buffer(&mut buffer, CHANNELS, &to_processor, &from_processor);

                match channels.to_output.push_interleaved(&buffer) {
                    PushStatus::Ok => {}
                    PushStatus::OutputNotReady => {}
                    PushStatus::OverflowOccurred { num_frames_pushed } => {
                        stream_error_sender
                            .try_send(Error::StreamPushOverflow {
                                name: "processor -> output",
                                frames: num_frames_pushed,
                            })
                            .ok();
                    }
                    PushStatus::UnderflowCorrected {
                        num_zero_frames_pushed,
                    } => {
                        stream_error_sender
                            .try_send(Error::StreamPushOverflow {
                                name: "processor -> output",
                                frames: num_zero_frames_pushed,
                            })
                            .ok();
                    }
                }
            }
        }

        // Sleep until the next update time.
        update_time += update_interval;
        let wait_time = update_time.saturating_duration_since(Instant::now());
        sleep(wait_time);
    }

    info!("Processor thread exiting");
}

fn initialize_streams(
    processor_sample_rate: usize,
    stream_error_sender: Sender<Error>,
) -> Result<(Streams, StreamChannels)> {
    let host = cpal::default_host();

    let Some(input_device) = host.default_input_device() else {
        return Err(Error::DefaultDeviceUnavailable { stream: "input" });
    };
    let input_config = input_device.default_input_config()?;
    debug!("default input config: {input_config:?}");
    let input_channels = input_config.channels() as usize;
    let input_sample_rate = input_config.sample_rate().0 as usize;
    let input_resampling_factor = (input_sample_rate as f64) / (processor_sample_rate as f64);
    let input_frames_per_update =
        ((FRAMES_PER_UPDATE as f64) * input_resampling_factor).ceil() as u32;
    let input_frames_per_update = match input_config.buffer_size() {
        SupportedBufferSize::Range { min, max } => input_frames_per_update.clamp(*min, *max),
        SupportedBufferSize::Unknown => input_frames_per_update,
    };

    let Some(output_device) = host.default_output_device() else {
        return Err(Error::DefaultDeviceUnavailable { stream: "output" });
    };
    let output_config = output_device.default_output_config()?;
    debug!("default output config: {output_config:?}");
    let output_channels = output_config.channels() as usize;
    let output_sample_rate = output_config.sample_rate().0 as usize;
    let output_resampling_factor = (output_sample_rate as f64) / (processor_sample_rate as f64);
    let output_frames_per_update =
        ((FRAMES_PER_UPDATE as f64) * output_resampling_factor).ceil() as u32;
    let output_frames_per_update = match output_config.buffer_size() {
        SupportedBufferSize::Range { min, max } => output_frames_per_update.clamp(*min, *max),
        SupportedBufferSize::Unknown => output_frames_per_update,
    };

    let input_update_duration = (input_frames_per_update as f64) / (input_sample_rate as f64);
    let processor_update_duration = (FRAMES_PER_UPDATE as f64) / (processor_sample_rate as f64);
    let output_update_duration = (output_frames_per_update as f64) / (output_sample_rate as f64);
    debug!(
        "input frames per update: {input_frames_per_update}, \
         output: {output_frames_per_update}"
    );
    debug!(
        "input update duration: {input_update_duration}, \
         output: {output_update_duration}, \
         processor: {processor_update_duration}"
    );

    // Set up the input -> processor channel
    let latency_seconds = input_update_duration.max(processor_update_duration) * 8.0;
    let capacity_seconds = latency_seconds * 16.0;
    let (input_to_processor_sender, input_to_processor_receiver) =
        resampling_channel::<f32, CHANNELS>(
            NonZeroUsize::new(CHANNELS).unwrap(),
            input_sample_rate as u32,
            processor_sample_rate as u32,
            ResamplingChannelConfig {
                latency_seconds,
                capacity_seconds,
                quality: ResampleQuality::Low,
                ..Default::default()
            },
        );

    // Set up the processor -> output channel
    let latency_seconds = processor_update_duration.max(output_update_duration) * 8.0;
    let capacity_seconds = latency_seconds * 16.0;
    let (processor_to_output_sender, processor_to_output_receiver) =
        resampling_channel::<f32, CHANNELS>(
            NonZeroUsize::new(CHANNELS).unwrap(),
            processor_sample_rate as u32,
            output_sample_rate as u32,
            ResamplingChannelConfig {
                latency_seconds,
                capacity_seconds,
                quality: ResampleQuality::Low,
                ..Default::default()
            },
        );

    let input_stream = initialize_input_stream(
        input_device,
        input_channels,
        input_sample_rate,
        input_frames_per_update,
        stream_error_sender.clone(),
        input_to_processor_sender,
    )?;

    let output_stream = initialize_output_stream(
        output_device,
        output_channels,
        output_sample_rate,
        output_frames_per_update,
        stream_error_sender,
        processor_to_output_receiver,
    )?;

    input_stream.play()?;
    output_stream.play()?;

    info!(
        "\
Audio stream started:
  input sample rate: {input_sample_rate}
  processor sample rate: {processor_sample_rate}
  output sample rate: {output_sample_rate}"
    );

    Ok((
        Streams {
            input: input_stream,
            input_config,
            output: output_stream,
            output_config,
        },
        StreamChannels {
            from_input: input_to_processor_receiver,
            to_output: processor_to_output_sender,
        },
    ))
}

fn initialize_input_stream(
    device: Device,
    channels: usize,
    sample_rate: usize,
    frames_per_update: u32,
    error_sender: Sender<Error>,
    mut input_to_processor: ResamplingProd<f32, 2>,
) -> Result<Stream> {
    let mut send_to_processor_fn =
        move |buffer: &[f32]| match input_to_processor.push_interleaved(&buffer) {
            PushStatus::Ok | PushStatus::OutputNotReady => {}
            PushStatus::OverflowOccurred { num_frames_pushed } => {
                error_sender
                    .try_send(Error::StreamPushOverflow {
                        name: "input -> processor",
                        frames: num_frames_pushed,
                    })
                    .ok();
            }
            PushStatus::UnderflowCorrected {
                num_zero_frames_pushed,
            } => {
                error_sender
                    .try_send(Error::StreamPushUnderflow {
                        name: "input -> processor",
                        frames: num_zero_frames_pushed,
                    })
                    .ok();
            }
        };

    let config = StreamConfig {
        channels: channels as u16,
        sample_rate: SampleRate(sample_rate as u32),
        buffer_size: BufferSize::Fixed(frames_per_update),
    };
    debug!("Setting up input stream with config: {config:?}");

    let mut buffer = vec![0.0; frames_per_update as usize * CHANNELS];
    let result = match channels {
        0 => {
            return Err(Error::DeviceHasNoAvailableChannels {
                device_name: device.name().unwrap_or_default(),
                stream: "input",
            });
        }
        1 => device.build_input_stream(
            &config,
            move |data: &[f32], _info: &InputCallbackInfo| {
                for (sample, buffer_frame) in data.iter().zip(buffer.chunks_exact_mut(CHANNELS)) {
                    buffer_frame.fill(*sample);
                }
                send_to_processor_fn(&buffer);
            },
            move |err| error!("Error on audio input stream: {}", err),
            None,
        ),
        2 => device.build_input_stream(
            &config,
            move |data: &[f32], _info: &InputCallbackInfo| {
                for (input_sample, buffer_sample) in data.iter().zip(buffer.iter_mut()) {
                    *buffer_sample = *input_sample;
                }

                send_to_processor_fn(&buffer);
            },
            move |err| error!("Error on audio input stream: {}", err),
            None,
        ),
        _ => device.build_input_stream(
            &config,
            move |data: &[f32], _info: &InputCallbackInfo| {
                for (input_frame, buffer_frame) in data
                    .chunks_exact(channels)
                    .zip(buffer.chunks_exact_mut(CHANNELS))
                {
                    for (input_sample, buffer_sample) in
                        input_frame.iter().zip(buffer_frame.iter_mut())
                    {
                        *buffer_sample = *input_sample;
                    }
                }

                send_to_processor_fn(&buffer);
            },
            move |err| error!("Error on audio input stream: {}", err),
            None,
        ),
    }?;

    Ok(result)
}

fn initialize_output_stream(
    device: Device,
    channels: usize,
    sample_rate: usize,
    frames_per_update: u32,
    error_sender: Sender<Error>,
    mut processor_to_output: ResamplingCons<f32>,
) -> Result<Stream> {
    let mut read_from_processor_fn =
        move |buffer: &mut [f32]| match processor_to_output.read_interleaved(buffer) {
            ReadStatus::Ok => {}
            ReadStatus::InputNotReady => {}
            ReadStatus::UnderflowOccurred { num_frames_read } => {
                error_sender
                    .try_send(Error::StreamReadUnderflow {
                        name: "processor -> output",
                        frames: num_frames_read,
                    })
                    .ok();
            }
            ReadStatus::OverflowCorrected {
                num_frames_discarded,
            } => {
                error_sender
                    .try_send(Error::StreamReadOverflow {
                        name: "processor -> output",
                        frames: num_frames_discarded,
                    })
                    .ok();
            }
        };

    let config = StreamConfig {
        channels: channels as u16,
        sample_rate: SampleRate(sample_rate as u32),
        buffer_size: BufferSize::Fixed(frames_per_update),
    };
    debug!("Setting up output stream with config: {config:?}");

    let mut buffer = vec![0.0; frames_per_update as usize * CHANNELS];
    let result = match channels {
        0 => {
            return Err(Error::DeviceHasNoAvailableChannels {
                device_name: device.name().unwrap_or_default(),
                stream: "output",
            });
        }
        1 => device.build_output_stream(
            &config,
            move |data: &mut [f32], _info: &OutputCallbackInfo| {
                read_from_processor_fn(&mut buffer);

                for (output_sample, processed_frame) in
                    data.iter_mut().zip(buffer.chunks_exact(CHANNELS))
                {
                    *output_sample = processed_frame.iter().copied().sum();
                }
            },
            move |err| error!("Error on audio output stream: {}", err),
            None,
        ),
        2 => device.build_output_stream(
            &config,
            move |data: &mut [f32], _info: &OutputCallbackInfo| read_from_processor_fn(data),
            move |err| error!("Error on audio output stream: {}", err),
            None,
        ),
        _ => device.build_output_stream(
            &config,
            move |data: &mut [f32], _info: &OutputCallbackInfo| {
                read_from_processor_fn(&mut buffer);

                for (output_frame, processed_frame) in data
                    .chunks_exact_mut(channels)
                    .zip(buffer.chunks_exact(CHANNELS))
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

    Ok(result)
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
    #[error("failed to send channel to processor thread")]
    ProcessorChannelSendFailed,

    #[error("{name} push underflow ({frames} frames)")]
    StreamPushUnderflow { name: &'static str, frames: usize },
    #[error("{name} push overflow ({frames} frames)")]
    StreamPushOverflow { name: &'static str, frames: usize },
    #[error("{name} read underflow ({frames} frames)")]
    StreamReadUnderflow { name: &'static str, frames: usize },
    #[error("{name} read overflow ({frames} frames)")]
    StreamReadOverflow { name: &'static str, frames: usize },

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
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
