use serde::{Deserialize, Serialize};
use crate::{enums::{SeasonStatus, FeedEventType, MaybeRecognized, Day}, feed_event::feed_event_text::FeedEventText};
use super::raw_feed_event::RawFeedEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    use std::{fs::File, io::Read, path::Path};

    use crate::feed_event::FeedEvent;


    #[test]
    fn round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = String::new(); 
        File::open(Path::new("test_data/s2_feed_event.json"))?.read_to_string(&mut buf)?;

        let json: serde_json::Value = serde_json::from_str(&buf)?;
        let event: FeedEvent = serde_json::from_value(json.clone())?;
        let round_trip = serde_json::to_value(&event)?; 
        assert_eq!(json, round_trip);
        Ok(())
    }
}
