use crate::AudioProcessor;

pub trait AudioModule {
    const NAME: &'static str;

    type Parameters: Parameters;
    type Processor: AudioProcessor;

    fn create_processor(sample_rate: usize) -> Self::Processor;
}

pub trait Parameters: Default {}
