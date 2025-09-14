pub(self) mod raw_team;
pub(self) mod team;

pub use team::{Team, TeamPlayer, TeamRecord, TeamPlayerCollection};
pub use crate::nom_parsing::parse_team_feed_event::parse_team_feed_event;
