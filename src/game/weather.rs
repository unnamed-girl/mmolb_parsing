use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Weather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String,

    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawWeather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawWeather> for Weather {
    fn from(value: RawWeather) -> Self {
        if value.extra_fields.len() > 0 {
            tracing::error!("Extra fields: {:?}", value.extra_fields)
        }
        Self { emoji: value.emoji, name: value.name, tooltip: value.tooltip, extra_fields: value.extra_fields }
    }
}
impl From<Weather> for RawWeather {
    fn from(value: Weather) -> Self {
        Self { emoji: value.emoji, name: value.name, tooltip: value.tooltip, extra_fields: value.extra_fields }
    }
}
