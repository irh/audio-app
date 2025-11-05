use crate::ToProcessor;

pub trait AudioProcessor: Sized + Send + Sync + 'static {
    type OutputMessage: Send;

    fn process_frame(&mut self, input: (f32, f32)) -> (f32, f32);
    fn process_buffer(
        &mut self,
        buffer: &mut [f32],
        channels: usize,
        on_output_message: impl FnMut(Self::OutputMessage),
    );
    fn receive_message(&mut self, command: ToProcessor);
}
