#[derive(Debug, Clone)]
pub enum ToProcessor {
    BeginEdit(usize),
    SetParameter(usize, f32),
    EndEdit(usize),
}

pub trait PushMessage<T: Send> {
    /// Pushes a message, exiting immediately if the channel is full.
    ///
    /// Returns `true` if the message was sent, and `false` otherwise.
    fn push(&self, message: T) -> bool;
}

pub trait PopMessage<T: Send> {
    /// Pops a message, exiting immediately if none is available.
    fn pop(&self) -> Option<T>;
}
