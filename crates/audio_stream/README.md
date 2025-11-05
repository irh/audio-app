# `audio_stream`

A stereo input/ouput audio stream for audio modules defined using the [`audio_module`](../audio_module) crate.

## Desktop / Mobile

`cpal` is used for cross-platform audio device support.

## Web

When compiled for `wasm32-unknown-unknown`, an audio graph is set up with a worklet that gets loaded with the `wasm` for a specific audio module. 
