use serde::{Deserialize, Serialize};

use crate::{enums::{Day, FeedEventStatus, FeedEventType, MaybeRecognized}, feed_event::{FeedEventText, FeedEvent}};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawFeedEvent {
    pub emoji: String,
    pub season: u8,
    pub day: MaybeRecognized<Day>,
    pub status: MaybeRecognized<FeedEventStatus>,
    pub text: FeedEventText,
    pub ts: String,
    #[serde(rename = "type")]
    pub event_type: MaybeRecognized<FeedEventType>,

    ///TODO
    links: serde_json::Value,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
impl From<RawFeedEvent> for FeedEvent {
    fn from(value: RawFeedEvent) -> Self {
        let RawFeedEvent { emoji, season, day, status, text, ts, event_type, extra_fields, links } = value;
        if extra_fields.len() > 0 {
            tracing::error!("Deserialization of FeedEvent found extra fields: {:?}", extra_fields)
        }
        FeedEvent { emoji, season, day, status, text, ts, event_type, extra_fields, links }
    }
}

impl From<FeedEvent> for RawFeedEvent {
    fn from(value: FeedEvent) -> Self {
        let FeedEvent { emoji, season, day, status, text, ts, event_type, extra_fields, links } = value;
        RawFeedEvent { emoji, season, day, status, text, ts, event_type, extra_fields, links }
    }
}