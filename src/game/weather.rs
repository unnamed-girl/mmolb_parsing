use serde::{Serialize, Deserialize};

use crate::utils::extra_fields_deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Weather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
