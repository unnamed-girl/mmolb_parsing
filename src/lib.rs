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
