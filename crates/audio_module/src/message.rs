#[derive(Debug, Clone)]
pub enum ToProcessor {
    BeginEdit(usize),
    SetParameter(usize, f32),
    EndEdit(usize),
}

pub trait PushMessage {
    fn push(&self, message: ToProcessor);
}

pub trait PopMessage<T: Send> {
    fn pop(&self) -> Option<T>;
}
