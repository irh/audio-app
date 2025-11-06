use std::sync::Arc;

use audio_module::{BoolParameter, FloatParameter, Parameter, PushMessage, ToProcessor};
use audio_stream::ToProcessorSender;
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

    #[persist = "editor-state"]
    pub editor_state: E::StateField,
}

impl<E: FreeverbEditor> FreeverbParams<E> {
    pub fn new(to_processor: ToProcessorSender) -> Self {
        let params = FreeverbParameters::default();

        Self {
            dampening: percent_parameter(params.dampening, to_processor.clone()),
            width: percent_parameter(params.width, to_processor.clone()),
            room_size: percent_parameter(params.room_size, to_processor.clone()),
            freeze: bool_parameter(params.freeze, to_processor.clone()),
            dry: percent_parameter(params.dry, to_processor.clone()),
            wet: percent_parameter(params.wet, to_processor.clone()),

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
            FreeverbParameterId::Scope => {}
        }
    }

    pub fn synchronize_ui_parameters(&self, ui_params: &mut FreeverbParameters) {
        ui_params.dampening.value = self.dampening.value();
        ui_params.width.value = self.width.value();
        ui_params.room_size.value = self.room_size.value();
        ui_params.freeze.value = self.freeze.value();
        ui_params.dry.value = self.dry.value();
        ui_params.wet.value = self.wet.value();
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

fn percent_parameter(param: FloatParameter, to_processor: ToProcessorSender) -> FloatParam {
    let id = param.id();

    FloatParam::new(
        param.name().to_string(),
        param.default_user_value(),
        FloatRange::Linear {
            min: param.value_converter().min(),
            max: param.value_converter().max(),
        },
    )
    .with_value_to_string(v2s_f32_percentage(2))
    .with_string_to_value(s2v_f32_percentage())
    .with_unit("%")
    .with_callback(Arc::new(move |value| {
        to_processor.push(ToProcessor::SetParameter(id, value));
    }))
}

fn bool_parameter(param: BoolParameter, to_processor: ToProcessorSender) -> BoolParam {
    let id = param.id();
    BoolParam::new(param.name().to_string(), param.default_user_value() != 0.0).with_callback(
        Arc::new(move |value| {
            to_processor.push(ToProcessor::SetParameter(id, if value { 1.0 } else { 0.0 }));
        }),
    )
}
