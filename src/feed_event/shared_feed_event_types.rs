use std::fmt::Display;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::enums::{BenchSlot, CelestialEnergyTier};
use crate::{
    enums::{Attribute, FeedEventType, ItemName, ItemPrefix, ItemSuffix},
    feed_event::FeedEvent,
    game_time::Breakpoints,
    parsed_event::Item,
    NotRecognized,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Error)]
pub enum FeedEventParseError {
    #[error("feed event type {} not recognized", .0.0)]
    EventTypeNotRecognized(#[source] NotRecognized),
    #[error("failed parsing {event_type} feed event \"{text}\"")]
    FailedParsingText {
        event_type: FeedEventType,
        text: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AttributeChange<S> {
    pub player_name: S,
    pub amount: i16,
    pub attribute: Attribute,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GreaterAugment {
    Headliners,
    StartSmall,
    Plating,
    LuckyDelivery,
    RestoreBackupRoster,
    RestoreBackupPitching,
    RestoreBackupBatting,
    RestoreBackupNullBatter,
    Training(BenchSlot),
    AugmentedInfield(i32),
    AugmentedRelief(i32),
    AugmentedStart(i32),
    AugmentedOutfield(i32),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PlayerGreaterAugment {
    Headliners { attribute: Attribute },
    StartSmall { attribute: Attribute },
    Plating,
    LuckyDelivery,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedDelivery<S> {
    pub player: S,
    pub item: Item<S>,
    pub discarded: Option<Item<S>>,
    pub equipped: bool,
}
impl<S: Display> FeedDelivery<S> {
    pub fn unparse(&self, event: &FeedEvent, delivery_label: &str) -> String {
        let FeedDelivery {
            player,
            item,
            discarded,
            equipped,
        } = self;

        let discarded = match discarded {
            Some(discarded) => {
                let verb = if Breakpoints::Season5TenseChange.before(
                    event.season as u32,
                    event.day.as_ref().ok().copied(),
                    None,
                ) {
                    "discarded"
                } else {
                    "discard"
                };

                format!(" They {verb} their {discarded}.")
            }
            None => String::new(),
        };

        let verb = if *equipped {
            "equips"
        } else if Breakpoints::Season5TenseChange.before(
            event.season as u32,
            event.day.as_ref().ok().copied(),
            None,
        ) {
            "received a"
        } else {
            "receives a"
        };

        let from = if *equipped { "from " } else { "" };

        format!("{player} {verb} {item} {from}{delivery_label}.{discarded}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EmojilessItem {
    pub prefix: Option<ItemPrefix>,
    pub item: ItemName,
    pub suffix: Option<ItemSuffix>,
}
impl Display for EmojilessItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let EmojilessItem {
            prefix,
            item,
            suffix,
        } = self;
        let prefix = match prefix {
            Some(prefix) => format!("{prefix} "),
            None => String::new(),
        };
        let suffix = match suffix {
            Some(suffix) => format!(" {suffix}"),
            None => String::new(),
        };

        write!(f, "{prefix}{item}{suffix}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FeedFallingStarOutcome {
    Injury,
    Infusion(CelestialEnergyTier),
    DeflectedHarmlessly,
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
            }
            FeedFallingStarOutcome::Infusion(infusion_tier) => match infusion_tier {
                CelestialEnergyTier::BeganToGlow => {
                    if event.before(Breakpoints::Season5TenseChange) {
                        format!("{player_name} began to glow brightly with celestial energy!")
                    } else {
                        format!("{player_name} begins to glow brightly with celestial energy!")
                    }
                }
                CelestialEnergyTier::Infused => {
                    format!("{player_name} {was_is} infused with a glimmer of celestial energy!")
                }
                CelestialEnergyTier::FullyCharged => format!(
                    "{player_name} {was_is} fully charged with an abundance of celestial energy!"
                ),
            },
            FeedFallingStarOutcome::DeflectedHarmlessly => {
                if event.before(Breakpoints::Season5TenseChange) {
                    format!("It deflected off {player_name} harmlessly.")
                } else {
                    format!("It deflects off {player_name} harmlessly.")
                }
            }
        }
    }
}
