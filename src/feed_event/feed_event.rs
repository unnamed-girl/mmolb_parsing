use serde::{Serialize, Deserialize};
use serde_with::serde_as;
use crate::{enums::{Day, FeedEventType, SeasonStatus}, feed_event::feed_event_text::FeedEventText, utils::{ExtraFields, MaybeRecognizedResult}};
use crate::utils::MaybeRecognizedHelper;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedEvent {
    pub emoji: String,
    pub season: u8,
    
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub day: MaybeRecognizedResult<Day>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub status: MaybeRecognizedResult<SeasonStatus>,
    pub text: FeedEventText,
    pub ts: String,
    #[serde(rename = "type")]
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub event_type: MaybeRecognizedResult<FeedEventType>,

    /// TODO
    pub(crate) links: serde_json::Value,

    #[serde(flatten)]
    pub extra_fields: ExtraFields,
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
