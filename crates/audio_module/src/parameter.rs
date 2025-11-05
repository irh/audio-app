use std::sync::Arc;

use crate::{
    BoolStringConverter,
    string_converter::{DefaultStringConverter, StringConverter},
    value_converter::{DefaultValueConverter, ValueConverter},
};

pub enum ValueType {
    Float,
    Bool,
}

pub trait Parameter {
    fn name(&self) -> Arc<String>;
    fn default_user_value(&self) -> f32;

    fn id(&self) -> usize;

    fn value_type(&self) -> ValueType;
    fn value_converter(&self) -> Arc<dyn ValueConverter> {
        Arc::new(DefaultValueConverter {})
    }

    fn string_converter(&self) -> Arc<dyn StringConverter>;
}

#[derive(Clone)]
pub struct BoolParameter {
    pub value: bool,
    id: usize,
    name: Arc<String>,
    default_user_value: bool,
    string_converter: Arc<dyn StringConverter>,
}

impl BoolParameter {
    pub fn new(name: &str, id: usize, default_value: bool) -> Self {
        Self {
            default_user_value: default_value,
            value: default_value,
            id,
            name: Arc::new(name.to_string()),
            string_converter: Arc::new(BoolStringConverter),
        }
    }
}

impl Parameter for BoolParameter {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> Arc<String> {
        self.name.clone()
    }

    fn default_user_value(&self) -> f32 {
        if self.default_user_value { 1.0 } else { 0.0 }
    }

    fn value_type(&self) -> ValueType {
        ValueType::Bool
    }

    fn string_converter(&self) -> Arc<dyn StringConverter> {
        self.string_converter.clone()
    }
}

impl PartialEq for BoolParameter {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.id == other.id
            && self.name == other.name
            && self.default_user_value == other.default_user_value
            && Arc::ptr_eq(&self.string_converter, &other.string_converter)
    }
}

pub struct FloatParameterBuilder {
    id: usize,
    name: String,
    default_user_value: f32,
    value_converter: Option<Arc<dyn ValueConverter>>,
    string_converter: Option<Arc<dyn StringConverter>>,
}

impl FloatParameterBuilder {
    pub fn default_user_value(mut self, default: f32) -> Self {
        self.default_user_value = default;
        self
    }

    pub fn with_value_converter(mut self, converter: impl ValueConverter + 'static) -> Self {
        self.value_converter = Some(Arc::new(converter));
        self
    }

    pub fn string_converter(mut self, converter: impl StringConverter + 'static) -> Self {
        self.string_converter = Some(Arc::new(converter));
        self
    }

    pub fn build(self) -> FloatParameter {
        FloatParameter {
            value: self.default_user_value,
            id: self.id,
            name: Arc::new(self.name),
            default_user_value: self.default_user_value,
            value_converter: self
                .value_converter
                .unwrap_or_else(|| Arc::new(DefaultValueConverter::default())),
            string_converter: self
                .string_converter
                .unwrap_or_else(|| Arc::new(DefaultStringConverter::default())),
        }
    }
}

#[derive(Clone)]
pub struct FloatParameter {
    pub value: f32,
    id: usize,
    name: Arc<String>,
    default_user_value: f32,
    value_converter: Arc<dyn ValueConverter>,
    string_converter: Arc<dyn StringConverter>,
}

impl FloatParameter {
    pub fn builder(name: &str, id: usize) -> FloatParameterBuilder {
        FloatParameterBuilder {
            id,
            name: name.to_string(),
            default_user_value: 0.0,
            value_converter: None,
            string_converter: None,
        }
    }
}

impl Parameter for FloatParameter {
    fn value_type(&self) -> ValueType {
        ValueType::Float
    }

    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> Arc<String> {
        self.name.clone()
    }

    fn default_user_value(&self) -> f32 {
        self.default_user_value
    }

    fn value_converter(&self) -> Arc<dyn ValueConverter> {
        self.value_converter.clone()
    }

    fn string_converter(&self) -> Arc<dyn StringConverter> {
        self.string_converter.clone()
    }
}

impl PartialEq for FloatParameter {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.id == other.id
            && self.name == other.name
            && self.default_user_value == other.default_user_value
            && Arc::ptr_eq(&self.value_converter, &other.value_converter)
            && Arc::ptr_eq(&self.string_converter, &other.string_converter)
    }
}
