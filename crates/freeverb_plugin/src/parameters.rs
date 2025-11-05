use freeverb_module::{FreeverbParameterId, FreeverbParameters};
use nih_plug::{
    formatters::{s2v_f32_percentage, v2s_f32_percentage},
    prelude::*,
};

use crate::FreeverbEditor;

#[derive(Params)]
pub struct FreeverbParams<E: FreeverbEditor> {
    #[id = "dampening"]
    pub dampening: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "room_size"]
    pub room_size: FloatParam,
    #[id = "freeze"]
    pub freeze: BoolParam,
    #[id = "dry"]
    pub dry: FloatParam,
    #[id = "wet"]
    pub wet: FloatParam,
    #[id = "scope"]
    pub scope_enabled: BoolParam,

    #[persist = "editor-state"]
    pub editor_state: E::StateField,
}

impl<E: FreeverbEditor> Default for FreeverbParams<E> {
    fn default() -> Self {
        Self {
            dampening: percent_parameter("Dampening", 0.5),
            width: percent_parameter("Width", 1.0),
            room_size: percent_parameter("Room Size", 0.5),
            freeze: BoolParam::new("Freeze", false),
            dry: percent_parameter("Dry", 0.0),
            wet: percent_parameter("Wet", 1.0 / 3.0),
            scope_enabled: BoolParam::new("Scope", true),

            editor_state: E::make_editor_state(),
        }
    }
}

impl<E: FreeverbEditor> FreeverbParams<E> {
    pub fn visit_parameter(&self, id: usize, visitor: &impl ParameterVisitor) {
        let Ok(parameter_id) = FreeverbParameterId::try_from(id) else {
            panic!("Invalid parameter ID: {id}");
        };

        match parameter_id {
            FreeverbParameterId::Dampening => visitor.visit(&self.dampening),
            FreeverbParameterId::Width => visitor.visit(&self.width),
            FreeverbParameterId::RoomSize => visitor.visit(&self.room_size),
            FreeverbParameterId::Freeze => visitor.visit(&self.freeze),
            FreeverbParameterId::Dry => visitor.visit(&self.dry),
            FreeverbParameterId::Wet => visitor.visit(&self.wet),
            FreeverbParameterId::Scope => visitor.visit(&self.scope_enabled),
        }
    }

    pub fn synchronize_ui_parameters(&self, ui_params: &mut FreeverbParameters) {
        ui_params.dampening.value = self.dampening.value();
        ui_params.width.value = self.width.value();
        ui_params.room_size.value = self.room_size.value();
        ui_params.freeze.value = self.freeze.value();
        ui_params.dry.value = self.dry.value();
        ui_params.wet.value = self.wet.value();
        ui_params.scope.value = self.scope_enabled.value();
    }
}

pub trait ParameterVisitor {
    fn visit<T: PlainFromF32>(&self, foo: &impl Param<Plain = T>);
}

pub trait PlainFromF32 {
    fn from_f32(value: f32) -> Self;
}

impl PlainFromF32 for f32 {
    fn from_f32(value: f32) -> Self {
        value
    }
}

impl PlainFromF32 for bool {
    fn from_f32(value: f32) -> Self {
        value != 0.0
    }
}

fn percent_parameter(name: impl Into<String>, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_value_to_string(v2s_f32_percentage(2))
        .with_string_to_value(s2v_f32_percentage())
        .with_unit("%")
}
