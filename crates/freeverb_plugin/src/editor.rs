use audio_module::{PushMessage, ToProcessor};
use nih_plug::{params::persist::PersistentField, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{Freeverb, FreeverbParams, ParameterVisitor, PlainFromF32};

pub trait FreeverbEditor: Send + Sized + 'static {
    type EditorState: Send + Sync + Serialize + for<'de> Deserialize<'de>;
    type StateField: for<'de> PersistentField<'de, Self::EditorState>;

    fn make_editor(freeverb: &Freeverb<Self>) -> Option<Box<dyn Editor>>;
    fn make_editor_state() -> Self::StateField;
}

pub trait FreeverbEditorState: Send + Sync + Serialize + for<'de> Deserialize<'de> {}

/// Used for when no editor is needed.
pub struct NoEditor;

impl FreeverbEditor for NoEditor {
    type EditorState = NoEditorState;
    type StateField = NoEditorState;

    fn make_editor(_freeverb: &Freeverb<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    fn make_editor_state() -> Self::EditorState {
        NoEditorState
    }
}

#[derive(Serialize, Deserialize)]
pub struct NoEditorState;

impl<'a> PersistentField<'a, NoEditorState> for NoEditorState {
    fn set(&self, _: NoEditorState) {}

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&NoEditorState) -> R,
    {
        f(&NoEditorState)
    }
}

pub struct CommandSetter<'a, 'b, E: FreeverbEditor> {
    setter: &'a ParamSetter<'b>,
    params: &'a FreeverbParams<E>,
}

impl<'a, 'b, E: FreeverbEditor> CommandSetter<'a, 'b, E> {
    pub fn new(setter: &'a ParamSetter<'b>, params: &'a FreeverbParams<E>) -> Self {
        Self { setter, params }
    }
}

impl<'a, 'b, E: FreeverbEditor> PushMessage for CommandSetter<'a, 'b, E> {
    fn push(&self, command: ToProcessor) {
        match command {
            ToProcessor::BeginEdit(id) => self
                .params
                .visit_parameter(id, &BeginEditVisitor(self.setter)),
            ToProcessor::SetParameter(id, value) => self.params.visit_parameter(
                id,
                &SetParameterVisitor {
                    setter: self.setter,
                    value,
                },
            ),
            ToProcessor::EndEdit(id) => self
                .params
                .visit_parameter(id, &EndEditVisitor(self.setter)),
        }
    }
}

struct BeginEditVisitor<'a, 'b>(&'a ParamSetter<'b>);

impl<'a, 'b> ParameterVisitor for BeginEditVisitor<'a, 'b> {
    fn visit<T>(&self, parameter: &impl Param<Plain = T>) {
        self.0.begin_set_parameter(parameter);
    }
}

struct SetParameterVisitor<'a, 'b> {
    setter: &'a ParamSetter<'b>,
    value: f32,
}

impl<'a, 'b> ParameterVisitor for SetParameterVisitor<'a, 'b> {
    fn visit<T: PlainFromF32>(&self, parameter: &impl Param<Plain = T>) {
        self.setter
            .set_parameter(parameter, T::from_f32(self.value));
    }
}

struct EndEditVisitor<'a, 'b>(&'a ParamSetter<'b>);

impl<'a, 'b> ParameterVisitor for EndEditVisitor<'a, 'b> {
    fn visit<T>(&self, parameter: &impl Param<Plain = T>) {
        self.0.end_set_parameter(parameter);
    }
}
