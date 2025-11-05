#[cfg_attr(not(target_arch = "wasm32"), path = "cpal.rs")]
#[cfg_attr(target_arch = "wasm32", path = "wasm.rs")]
mod audio_stream;

pub use crate::audio_stream::*;
