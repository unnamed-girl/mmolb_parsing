use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use crate::{enums::{CelestialEnergyTier, Day, FeedEventType, LinkType, SeasonStatus}, utils::{extra_fields_deserialize, MaybeRecognizedHelper, MaybeRecognizedResult, TimestampHelper}};

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FeedFallingStarOutcome {
    Injury,
    Infusion(CelestialEnergyTier),
    DeflectedHarmlessly
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
