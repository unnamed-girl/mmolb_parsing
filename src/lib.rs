#![allow(clippy::module_inception)]

pub(crate) mod time;
pub(crate) mod utils;

pub mod enums;
pub mod feed_event;
pub mod game;
pub mod nom_parsing;
pub mod parsed_event;
pub mod parsing;
pub mod player;
pub mod player_feed;
pub mod team;
pub mod team_feed;

pub use game::Game;
pub use parsed_event::ParsedEventMessage;
pub use parsing::{process_event, process_game};

pub use utils::{
    AddedLater, AddedLaterResult, EmptyArrayOr, MaybeRecognizedResult, NotRecognized, RemovedLater,
    RemovedLaterResult,
};

use crate::{enums::Day, parsed_event::EmojiTeam, time::Time};

#[derive(Clone, Copy)]
pub struct UnparsingContext<'a> {
    pub season: u32,
    pub day: Option<Day>,
    pub away_emoji_team: EmojiTeam<&'a str>,
    pub home_emoji_team: EmojiTeam<&'a str>,
}

impl<'a> From<&'a Game> for UnparsingContext<'a> {
    fn from(value: &'a Game) -> Self {
        let Game {
            season,
            day,
            away_team_emoji,
            away_team_name,
            home_team_emoji,
            home_team_name,
            ..
        } = value;
        UnparsingContext {
            season: *season,
            day: day.as_ref().ok().copied(),
            away_emoji_team: EmojiTeam {
                emoji: away_team_emoji,
                name: away_team_name,
            },
            home_emoji_team: EmojiTeam {
                emoji: home_team_emoji,
                name: home_team_name,
            },
        }
    }
}

impl<'a> UnparsingContext<'a> {
    /// Whether this event is before the given time
    pub(crate) fn before(&self, event_index: Option<u16>, time: impl Into<Time>) -> bool {
        time.into().before(self.season, self.day, event_index)
    }

    /// Whether this event is after the given time
    pub(crate) fn after(&self, event_index: Option<u16>, time: impl Into<Time>) -> bool {
        time.into().after(self.season, self.day, event_index)
    }
}