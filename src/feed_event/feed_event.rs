use serde::{Serialize, Deserialize};
use crate::{enums::{SeasonStatus, FeedEventType, MaybeRecognized, Day}, feed_event::feed_event_text::FeedEventText};
use super::raw_feed_event::RawFeedEvent;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(from = "RawFeedEvent", into = "RawFeedEvent")]
pub struct FeedEvent {
    pub emoji: String,
    pub season: u8,
    pub day: MaybeRecognized<Day>,
    pub status: MaybeRecognized<SeasonStatus>,
    pub text: FeedEventText,
    pub ts: String,
    pub event_type: MaybeRecognized<FeedEventType>,

    /// TODO
    pub(crate) links: serde_json::Value,

    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{feed_event::FeedEvent, utils::assert_round_trip};


    #[test]
    #[tracing_test::traced_test]
    fn feed_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        assert_round_trip::<FeedEvent>(Path::new("test_data/s2_feed_event.json"))?;
        assert!(!logs_contain("not recognized"));
        Ok(())
    }
}
