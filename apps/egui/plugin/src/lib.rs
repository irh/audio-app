use audio_module::{PushMessage, ToProcessor};
use audio_stream::{FromProcessorReceiver, ToProcessorSender};
use freeverb_module::{FreeverbParameterId, FreeverbProcessor};
use freeverb_plugin::*;
use nih_plug::prelude::*;
use nih_plug_egui::{EguiState, create_egui_editor, egui::CentralPanel};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use ui_egui::{FreeverbUi, FreeverbUiState};

#[derive(Default)]
pub struct EguiEditor {
    ui_state: FreeverbUiState,
}

impl FreeverbEditor for EguiEditor {
    type EditorState = EguiState;
    type StateField = Arc<EguiState>;

    fn make_editor(
        freeverb: &Freeverb<Self>,
        sample_rate: Arc<AtomicUsize>,
        to_processor: ToProcessorSender,
        from_processor: FromProcessorReceiver<FreeverbProcessor>,
    ) -> Option<Box<dyn Editor>> {
        let plugin_params = freeverb.params.clone();
        let egui_state = freeverb.params.editor_state.clone();

        let mut editor = EguiEditor::default();

        // Enable the scope
        to_processor.push(ToProcessor::SetParameter(
            FreeverbParameterId::Scope as usize,
            1.0,
        ));
        editor.ui_state.parameters.scope.value = true;

        create_egui_editor(
            egui_state.clone(),
            editor,
            |_, _| {},
            move |egui_ctx, setter, editor| {
                editor.ui_state.sample_rate = sample_rate.load(Ordering::Relaxed);
                editor.ui_state.receive_processor_messages(&from_processor);

                // Apply any host updates to the ui parameters
                plugin_params.synchronize_ui_parameters(&mut editor.ui_state.parameters);

                // Render the UI
                CentralPanel::default().show(egui_ctx, |ui| {
                    let to_processor = ToProcessorForParams::new(setter, &plugin_params);
                    ui.add(FreeverbUi::new(&mut editor.ui_state, Some(to_processor)));
                });
            },
        )
    }

    fn make_editor_state() -> Self::StateField {
        EguiState::from_size(385, 230)
    }
}

#[cfg(feature = "clap")]
nih_export_clap!(Freeverb<EguiEditor>);

#[cfg(feature = "vst3")]
nih_export_vst3!(Freeverb<EguiEditor>);
