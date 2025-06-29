use serde::{Deserialize, Serialize};
use crate::{enums::{FeedEventStatus, FeedEventType, MaybeRecognized, FeedEventDay}, feed_event::feed_event_text::FeedEventText};
use super::raw_feed_event::RawFeedEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "RawFeedEvent", into = "RawFeedEvent")]
pub struct FeedEvent {
    pub emoji: String,
    pub season: u8,
    pub day: MaybeRecognized<FeedEventDay>,
    pub status: MaybeRecognized<FeedEventStatus>,
    pub text: FeedEventText,
    pub ts: String,
    pub event_type: MaybeRecognized<FeedEventType>,
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
