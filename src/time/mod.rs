use crate::enums::PollenCount;
use crate::utils::SometimesMissingHelper;
use crate::AddedLaterResult;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Time {
    pub season_number: u32,
    pub season_day: u32,

    // TODO the key to this is related to SeasonStatus, but it apparently
    //   doesn't deserialize the same
    pub season_status: String,

    // TODO the key to this is related to SeasonStatus, but it apparently
    //   doesn't deserialize the same
    pub phase_times: HashMap<String, chrono::DateTime<chrono::FixedOffset>>,

    #[serde_as(as = "SometimesMissingHelper<_>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub pollen_level: AddedLaterResult<PollenCount>,
}
