use freeverb_plugin::*;
use nih_plug::prelude::*;
use nih_plug_egui::{EguiState, create_egui_editor, egui::CentralPanel};
use std::sync::Arc;
use ui_egui::{FreeverbUi, FreeverbUiState};

pub struct EguiEditor {
    ui_state: FreeverbUiState,
}

impl FreeverbEditor for EguiEditor {
    type EditorState = EguiState;
    type StateField = Arc<EguiState>;

    fn make_editor(freeverb: &Freeverb<Self>) -> Option<Box<dyn Editor>> {
        let plugin_params = freeverb.params.clone();
        let egui_state = freeverb.params.editor_state.clone();

        create_egui_editor(
            egui_state.clone(),
            EguiEditor {
                ui_state: FreeverbUiState::default(),
            },
            |_, _| {},
            move |egui_ctx, setter, editor| {
                // Apply any host updates to the ui parameters
                plugin_params.synchronize_ui_parameters(&mut editor.ui_state.parameters);

                // Render the UI
                CentralPanel::default().show(egui_ctx, |ui| {
                    let commands = CommandSetter::new(setter, &plugin_params);
                    ui.add(FreeverbUi::new(&mut editor.ui_state, &mut Some(commands)));
                });
            },
        )
    }

    fn make_editor_state() -> Self::StateField {
        EguiState::from_size(170, 230)
    }
}

#[cfg(feature = "clap")]
nih_export_clap!(Freeverb<EguiEditor>);

#[cfg(feature = "vst3")]
nih_export_vst3!(Freeverb<EguiEditor>);
