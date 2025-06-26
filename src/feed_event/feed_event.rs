use serde::{Deserialize, Serialize};
use crate::{enums::{FeedEventStatus, FeedEventType, MaybeRecognized, FeedEventDay}, feed_event::feed_event_text::FeedEventText};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeedEvent {
    pub emoji: String,
    pub season: u8,
    pub day: MaybeRecognized<FeedEventDay>,
    pub status: MaybeRecognized<FeedEventStatus>,
    pub text: FeedEventText,
    pub ts: String,
    #[serde(rename = "type")]
    pub event_type: MaybeRecognized<FeedEventType>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
