mod editor;
mod parameters;
mod plugin;

pub use crate::{editor::*, parameters::*, plugin::*};

#[cfg(all(feature = "export_no_editor", feature = "clap"))]
nih_plug::nih_export_clap!(Freeverb<NoEditor>);

#[cfg(all(feature = "export_no_editor", feature = "vst3"))]
nih_plug::nih_export_vst3!(Freeverb<NoEditor>);
