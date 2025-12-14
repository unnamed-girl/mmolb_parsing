//! A note on implementation:
//! For a lot of this code I will have a function return a parser instead of the function just being a parser.
//! This is because it makes it easier to inject context later when I inevitably need to use a timestamp to choose which parser to use

pub(crate) mod shared;
pub(crate) mod parse;
pub(crate) mod parse_player_feed_event;
pub(crate) mod parse_team_feed_event;

pub use shared::ParsingContext;
pub use parse::parse_event;
