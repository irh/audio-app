use crate::{PopMessage, PushMessage, ToProcessor};

pub trait AudioProcessor: Sized + Send + 'static {
    type OutputMessage: Send;

    fn process_buffer<To, From>(
        &mut self,
        buffer: &mut [f32],
        channels: usize,
        to_processor: &To,
        from_processor: &From,
    ) where
        To: PopMessage<ToProcessor>,
        From: PushMessage<Self::OutputMessage>;
}
