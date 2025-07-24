use time::{OffsetDateTime};
use serde::{Serialize, Deserialize};
use serde_with::serde_as;
use crate::{enums::{Day, FeedEventType, SeasonStatus}, utils::{extra_fields_deserialize, MaybeRecognizedResult}};
use crate::utils::MaybeRecognizedHelper;

// From https://time-rs.github.io/book/api/well-known-format-descriptions.html#rfc-3339, as time's default rfc3339 implementation automatically converts "+00:00" to "Z"
time::serde::format_description!(mmolb_timestamp_format, OffsetDateTime, "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]+[offset_hour]:[offset_minute]");

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
    #[serde(rename = "ts", with = "mmolb_timestamp_format")]
    pub timestamp: OffsetDateTime,
    #[serde(rename = "type")]
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub event_type: MaybeRecognizedResult<FeedEventType>,

    /// TODO
    pub(crate) links: serde_json::Value,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{feed_event::FeedEvent, utils::{assert_round_trip, no_tracing_errs}};


    #[test]
    fn feed_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();

        assert_round_trip::<FeedEvent>(Path::new("test_data/s2_feed_event.json"))?;
        
        drop(no_tracing_errs);
        Ok(())
    }
}
