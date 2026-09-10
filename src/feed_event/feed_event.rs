use crate::utils::{
    extra_fields_deserialize, MaybeRecognizedHelper, SometimesMissingHelper, TimestampHelper,
};
use crate::{
    enums::{Day, FeedEventType, LinkType, SeasonStatus},
    utils::MaybeRecognizedResult,
    AddedLaterResult,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    EnumIter,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    IntoStaticStr,
    Display,
)]
#[serde(rename_all = "snake_case")]
pub enum FeedEventCollection {
    Player,
    Team,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedEventLegacySource {
    collection: FeedEventCollection,
    source_id: String,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedEvent {
    pub emoji: String,
    pub season: u8,

    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub day: MaybeRecognizedResult<Day>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub status: MaybeRecognizedResult<SeasonStatus>,
    pub text: String,
    #[serde(rename = "ts")]
    #[serde_as(as = "TimestampHelper")]
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "type")]
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub event_type: MaybeRecognizedResult<FeedEventType>,

    pub links: Vec<Link>,

    // Added in s11
    #[serde_as(as = "SometimesMissingHelper<_>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub _id: AddedLaterResult<String>,
    #[serde_as(as = "SometimesMissingHelper<_>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub legacy_event_key: AddedLaterResult<String>,
    #[serde_as(as = "SometimesMissingHelper<_>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub legacy_source: AddedLaterResult<FeedEventLegacySource>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Link {
    pub id: String,
    #[serde(rename = "type")]
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub link_type: MaybeRecognizedResult<LinkType>,
    pub index: Option<u16>,
    #[serde(rename = "match")]
    pub link_match: String,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{
        feed_event::FeedEvent,
        utils::{assert_round_trip, no_tracing_errs},
    };

    #[test]
    fn feed_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();

        assert_round_trip::<FeedEvent>(Path::new("test_data/s2_feed_event.json"))?;

        drop(no_tracing_errs);
        Ok(())
    }
}
