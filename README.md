# `audio-app`

This project exists explore how different Rust GUI / App frameworks work in the context of building audio applications.

Each app includes a simple UI for the [freeverb](https://github.com/irh/freeverb-rs) processor,
and where possible includes a realtime visualization of the processed audio.

## Goals

Each app should:
- ...be written idiomatically for the specific framework, while making use of the parameters exposed by [the freeverb module](`./crates/freeverb_module`).
- ...have roughly the same overall layout (although there's no need to make them look identical).
- ...be simple enough to follow as an introduction to the framework.

## Repo Structure

### [`crates/`](./crates)

Supporting crates used by the various app projects.

### [`apps/`](./apps)

The `apps/` directory contains subdirectories for each framework.

See each app's `README.md` for more information.

### [`xtask/`](./xtask)

A `cargo xtask` command is included that currently defers to `nih_plug::xtask` for bundling plugins.

## Supported Frameworks

| Framework | Version | Desktop | iPhone | Android | Web | Plugin |
| --------- | ------- | ------- | ------ | ------- | --- | ------ |
| [Dioxus][dioxus] | `0.17`      | ✅ | ✅ | ✅ |✅ | ❌ |
| [egui][egui]     | `0.33`      | ✅ | ✅ | ✅ |✅ | ✅ |
| [iced][iced]     | `0.13.1`    | ✅ | ❌ | ❌ |❌ | ❌ |
| [Vizia][vizia]   | `c0ada337a` | ✅ | ❌ | ❌ |❌ | ❌ |

### Notes

- Desktop builds have only been tested so far on macOS.
- Android builds require a minimum of API 26.
- Plugin support is currently underdeveloped.
  - `nih_plug` currently supports older versions of `egui`, `iced`, and `vizia`. Plugin support for `egui` is enabled via a fork of `nih_plug` that supports `v0.33`. Support for `iced` and `vizia` should be possible, but will either require further patching.

## Contributing

Contributions are very welcome!
- If a framework isn't represented here then it would be great to include it.
- Updates for new framework versions are very welcome.
- Testing on various platforms is valuable.
- Documentation to make the examples easier to understand for newcomers is very welcome.

[dioxus]: https://dioxuslabs.com
[egui]: https://www.egui.rs
[iced]: https://iced.rs
[vizia]: https://github.com/vizia/vizia
