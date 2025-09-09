use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{feed_event::{FeedEvent, FeedEventParseError}, utils::extra_fields_deserialize};


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamFeed {
    pub feed: Vec<FeedEvent>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParsedTeamFeedEventText<S> {
    ParseError {
        error: FeedEventParseError,
        text: S
    },
}

impl<S: Display> ParsedTeamFeedEventText<S> {
    pub fn unparse(&self, event: &FeedEvent) -> String {
        match self {
            ParsedTeamFeedEventText::ParseError { error: _, text } => text.to_string(),
        }
    }
}
