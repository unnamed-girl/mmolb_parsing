use std::fmt::Display;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use crate::{enums::{CelestialEnergyTier, Day, FeedEventType, LinkType, SeasonStatus}, utils::{extra_fields_deserialize, MaybeRecognizedHelper, MaybeRecognizedResult, TimestampHelper}};
use crate::time::Breakpoints;

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

impl FeedFallingStarOutcome {
    pub fn unparse<S: Display>(&self, event: &FeedEvent, player_name: S) -> String {
        let was_is = if event.before(Breakpoints::Season5TenseChange) {
            "was"
        } else {
            "is"
        };
        
        match self {
            FeedFallingStarOutcome::Injury => {
                if event.after(Breakpoints::EternalBattle) {
                    format!("{player_name} {was_is} injured by the extreme force of the impact!")
                } else {
                    format!("{player_name} {was_is} hit by a Falling Star!")
                }
            },
            FeedFallingStarOutcome::Infusion(infusion_tier) => {
                match infusion_tier {
                    CelestialEnergyTier::BeganToGlow => if event.before(Breakpoints::Season5TenseChange) {
                        format!("{player_name} began to glow brightly with celestial energy!")
                    } else {
                        format!("{player_name} begins to glow brightly with celestial energy!")
                    },
                    CelestialEnergyTier::Infused => format!("{player_name} {was_is} infused with a glimmer of celestial energy!"),
                    CelestialEnergyTier::FullyCharged => format!("{player_name} {was_is} fully charged with an abundance of celestial energy!"),
                }
            },
            FeedFallingStarOutcome::DeflectedHarmlessly => if event.before(Breakpoints::Season5TenseChange) {
                format!("It deflected off {player_name} harmlessly.")
            } else {
                format!("It deflects off {player_name} harmlessly.")
            }
        }

    }
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
