use serde::{Serialize, Deserialize};

use crate::utils::ExtraFields;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Weather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String,

    #[serde(flatten)]
    pub extra_fields: ExtraFields,
}
