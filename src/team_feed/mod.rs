use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{enums::{Attribute, ModificationType, Slot}, feed_event::{AttributeChange, EmojilessItem, FeedDelivery, FeedEvent, FeedEventParseError}, parsed_event::EmojiTeam, time::Breakpoints, utils::extra_fields_deserialize};


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
    GameResult {
        /// Sometimes this name is wrong: early season 1 bug where the events didn't have spaces between words.
        home_team: EmojiTeam<S>,
        /// Sometimes this name is wrong: early season 1 bug where the events didn't have spaces between words.
        away_team: EmojiTeam<S>,

        home_score: u8,
        away_score: u8
    },
    Delivery {
        delivery: FeedDelivery<S>
    },
    Shipment {
        delivery: FeedDelivery<S>
    },
    SpecialDelivery {
        delivery: FeedDelivery<S>
    },
    PhotoContest {
        tokens_earned: u16
    },
    Prosperous {
        team: EmojiTeam<S>,
        tokens_earned: u16
    },
    Enchantment {
        player_name: S,
        item: EmojilessItem,
        amount: u8,
        attribute: Attribute,
        enchant_two: Option<(u8, Attribute)>,
        compensatory: bool
    },
    AttributeChanges {
        changes: Vec<AttributeChange<S>>
    },
    AttributeEquals {
        players: Vec<(Option<Slot>, S)>,
        changing_attribute: Attribute,
        value_attribute: Attribute,
    },
    SwapPlaces {
        player_one: S,
        player_two: S,
    },
    Modification {
        player_name: S,
        lost_modification: Option<ModificationType>,
        modification: ModificationType
    },
}

impl<S: Display> ParsedTeamFeedEventText<S> {
    pub fn unparse(&self, event: &FeedEvent) -> String {
        match self {
            ParsedTeamFeedEventText::ParseError { error: _, text } => text.to_string(),
            ParsedTeamFeedEventText::GameResult { home_team, away_team, home_score, away_score } => {
                format!("{} vs. {} - FINAL {}-{}", away_team, home_team, away_score, home_score)
            }
            ParsedTeamFeedEventText::Delivery { delivery } => delivery.unparse("Delivery"),
            ParsedTeamFeedEventText::SpecialDelivery { delivery } => delivery.unparse("Special Delivery"),
            ParsedTeamFeedEventText::Shipment { delivery } => delivery.unparse("Shipment"),
            ParsedTeamFeedEventText::PhotoContest { tokens_earned } => {
                format!("Earned {tokens_earned} 🪙 in the Photo Contest.")
            }
            ParsedTeamFeedEventText::Prosperous { team, tokens_earned } => {
                format!("{team} are Prosperous! They earned {tokens_earned} 🪙.")
            }
            ParsedTeamFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two, compensatory } => {
                if event.before(Breakpoints::Season1EnchantmentChange) {
                    if enchant_two.is_some() {
                        tracing::error!("Season 1 enchantment had two enchants");
                    }
                    if *compensatory {
                        tracing::error!("Season 1 enchantment was compensatory")
                    }
                    format!("{player_name}'s {item} was enchanted with +{amount} to {attribute}.")
                } else if event.before(Breakpoints::season(2)) {
                    if enchant_two.is_some() {
                        tracing::error!("Season 1 enchantment had two enchants");
                    }
                    if *compensatory {
                        tracing::error!("Season 1 enchantment was compensatory")
                    }
                    format!("The Item Enchantment was a success! {player_name}'s {item} gained a +{amount} {attribute} bonus.")
                } else {
                    let enchant_type = compensatory.then_some("Compensatory").unwrap_or("Item");
                    match enchant_two {
                        Some((amount_two, attribute_two)) => format!("The {enchant_type} Enchantment was a success! {player_name}'s {item} was enchanted with +{amount} {attribute} and +{amount_two} {attribute_two}."),
                        None =>  format!("The {enchant_type} Enchantment was a success! {player_name}'s {item} gained a +{amount} {attribute} bonus.")
                    }
                }
            },
            ParsedTeamFeedEventText::AttributeChanges { changes } => {
                changes.iter()
                    .map(|change|  format!("{} gained +{} {}.", change.player_name, change.amount, change.attribute))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            ParsedTeamFeedEventText::AttributeEquals { players, changing_attribute, value_attribute } => {
                if Breakpoints::Season3.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                    let intro = format!("Batters' {changing_attribute} was set to their {value_attribute}. Lineup:");
                    let lineup = players.into_iter()
                        .enumerate()
                        .map(|(i, (slot, p))| format!(" {}. {} {p}", i+1, slot.as_ref().map(Slot::to_string).unwrap_or_default()))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("{intro}{lineup}")
                } else {
                    let f = |player_name: &S, changing_attribute: &Attribute, value_attribute: &Attribute,| {
                        if Breakpoints::S1AttributeEqualChange.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                            format!("{}'s {} became equal to their current base {}.", player_name, changing_attribute, value_attribute)
                        } else {
                            format!("{}'s {} became equal to their base {}.", player_name, changing_attribute, value_attribute)
                        }
                    };
                    players.into_iter()
                        .map(|(_, p)| f(p, changing_attribute, value_attribute))
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            }
        }
    }
}
