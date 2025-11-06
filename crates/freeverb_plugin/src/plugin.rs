use crate::{FreeverbEditor, FreeverbParams};
use audio_module::AudioProcessor;
use audio_stream::{
    FromProcessorReceiver, FromProcessorSender, ToProcessorReceiver, ToProcessorSender,
};
use freeverb_module::FreeverbProcessor;
use nih_plug::prelude::*;
use std::{
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub struct Freeverb<E: FreeverbEditor> {
    pub params: Arc<FreeverbParams<E>>,
    to_processor_sender: ToProcessorSender,
    to_processor_receiver: ToProcessorReceiver,
    from_processor_sender: FromProcessorSender<FreeverbProcessor>,
    from_processor_receiver: FromProcessorReceiver<FreeverbProcessor>,
    initialized: Option<InitializedState>,
    sample_rate: Arc<AtomicUsize>,
    _editor: PhantomData<E>,
}

struct InitializedState {
    processor: FreeverbProcessor,
    process_buffer: Vec<f32>,
}

impl<E: FreeverbEditor> Default for Freeverb<E> {
    fn default() -> Self {
        let channel_capacity = 1024;

        let (sender, receiver) = crossbeam_channel::bounded(channel_capacity);
        let to_processor_sender = ToProcessorSender::new(sender);
        let to_processor_receiver = ToProcessorReceiver::new(receiver);

        let (sender, receiver) = crossbeam_channel::bounded(channel_capacity);
        let from_processor_sender = FromProcessorSender::new(sender);
        let from_processor_receiver = FromProcessorReceiver::new(receiver);

        Self {
            params: Arc::new(FreeverbParams::new(to_processor_sender.clone())),
            to_processor_sender,
            to_processor_receiver,
            from_processor_sender,
            from_processor_receiver,
            initialized: None,
            sample_rate: Arc::default(),
            _editor: PhantomData,
        }
    }
}

impl<E: FreeverbEditor> Plugin for Freeverb<E> {
    const NAME: &'static str = "Freeverb";
    const VENDOR: &'static str = env!("VENDOR_NAME");
    const URL: &'static str = env!("VENDOR_URL");
    const EMAIL: &'static str = env!("VENDOR_EMAIL");
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        E::make_editor(
            self,
            self.sample_rate.clone(),
            self.to_processor_sender.clone(),
            self.from_processor_receiver.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sample_rate = buffer_config.sample_rate as usize;
        self.initialized = Some(InitializedState {
            processor: FreeverbProcessor::new(sample_rate),
            process_buffer: vec![0.0; buffer_config.max_buffer_size as usize * 2],
        });

        self.sample_rate.store(sample_rate, Ordering::Relaxed);

        true
    }

    fn process(
        &mut self,
        host_buffers: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let Some(InitializedState {
            processor,
            process_buffer,
            ..
        }) = &mut self.initialized
        else {
            return ProcessStatus::Error("Uninitialized");
        };

        // Interleave the contents of the host buffers into the process buffer
        process_buffer.clear();
        process_buffer.extend(host_buffers.iter_samples().flatten().map(|sample| *sample));

        processor.process_buffer(
            process_buffer,
            2,
            &self.to_processor_receiver,
            &self.from_processor_sender,
        );

        // Deinterleave the process buffer into the host buffers
        for (processed_sample, host_sample) in process_buffer
            .iter()
            .zip(host_buffers.iter_samples().flatten())
        {
            *host_sample = *processed_sample;
        }

        ProcessStatus::KeepAlive
    }
}

#[cfg(feature = "vst3")]
impl<E: FreeverbEditor> ClapPlugin for Freeverb<E> {
    const CLAP_ID: &'static str = env!("PRODUCT_ID");
    const CLAP_DESCRIPTION: Option<&'static str> = Some("The Freeverb reverb");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Reverb,
    ];
}

#[cfg(feature = "vst3")]
impl<E: FreeverbEditor> Vst3Plugin for Freeverb<E> {
    const VST3_CLASS_ID: [u8; 16] = *b"0123456789abcdef";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
