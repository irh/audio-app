pub trait StringConverter: Send + Sync {
    fn to_string(&self, value: f32) -> String;
    fn to_string_with_unit(&self, value: f32) -> String {
        self.to_string(value)
    }

    fn to_f32(&self, s: &str) -> Option<f32>;

    fn unit(&self) -> Option<&str> {
        None
    }
}

#[derive(Default, Clone)]
pub struct DefaultStringConverter {
    unit: String,
}

impl DefaultStringConverter {
    pub fn new(unit: impl Into<String>) -> Self {
        Self { unit: unit.into() }
    }
}

impl StringConverter for DefaultStringConverter {
    fn to_string(&self, value: f32) -> String {
        format!("{:.0}", value)
    }

    fn to_string_with_unit(&self, value: f32) -> String {
        format!("{:.0} {}", value, self.unit)
    }

    fn to_f32(&self, s: &str) -> Option<f32> {
        s.parse().ok()
    }

    fn unit(&self) -> Option<&str> {
        Some(self.unit.as_str())
    }
}

#[derive(Default, Clone)]
pub struct PercentStringConverter {}

impl StringConverter for PercentStringConverter {
    fn to_string(&self, value: f32) -> String {
        format!("{:.0}", value * 100.0)
    }

    fn to_string_with_unit(&self, value: f32) -> String {
        format!("{:.0} %", value * 100.0)
    }

    fn to_f32(&self, s: &str) -> Option<f32> {
        s.parse::<f32>().ok().map(|n| n / 100.0)
    }

    fn unit(&self) -> Option<&str> {
        Some("%")
    }
}

#[derive(Default, Clone)]
pub struct BoolStringConverter;

impl StringConverter for BoolStringConverter {
    fn to_string(&self, value: f32) -> String {
        if value == 0.0 { "off" } else { "on" }.to_string()
    }

    fn to_f32(&self, s: &str) -> Option<f32> {
        match s {
            "off" => Some(0.0),
            "on" => Some(1.0),
            _ => None,
        }
    }
}
