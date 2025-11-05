use crate::{FreeverbEditor, FreeverbParams};
use nih_plug::prelude::*;
use std::{marker::PhantomData, sync::Arc};

pub struct Freeverb<E: FreeverbEditor> {
    pub params: Arc<FreeverbParams<E>>,
    freeverb: Option<freeverb::Freeverb<f32>>,
    _editor: PhantomData<E>,
}

impl<E: FreeverbEditor> Default for Freeverb<E> {
    fn default() -> Self {
        Self {
            params: Arc::new(FreeverbParams::default()),
            freeverb: None,
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
        E::make_editor(self)
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.freeverb = Some(freeverb::Freeverb::new(buffer_config.sample_rate as usize));
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let Some(freeverb) = &mut self.freeverb else {
            return ProcessStatus::Error("Uninitialized");
        };

        freeverb.set_dampening(self.params.dampening.value());
        freeverb.set_width(self.params.width.value());
        freeverb.set_room_size(self.params.room_size.value());
        freeverb.set_freeze(self.params.freeze.value());
        freeverb.set_dry(self.params.dry.value());
        freeverb.set_wet(self.params.wet.value());

        for mut frame in buffer.iter_samples() {
            let mut samples = frame.iter_mut();
            let left = samples.next().unwrap();
            let right = samples.next().unwrap();
            let (out_left, out_right) = freeverb.tick((*left, *right));
            *left = out_left;
            *right = out_right;
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
