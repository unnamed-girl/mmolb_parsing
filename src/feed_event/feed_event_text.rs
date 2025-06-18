use std::{fmt::Display, ops::{Deref, DerefMut}};

use serde::{Deserialize, Serialize};

use crate::{enums::{FeedEventType, Item, Attribute}, nom_parsing::parse_feed_event};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeedEventText(pub String);

impl FeedEventText {
    pub fn parse(&self, event_type: FeedEventType) -> ParsedFeedEventText<&str> {
        parse_feed_event(self, event_type)
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for FeedEventText {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for FeedEventText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParsedFeedEventText<S> {
    ParseError {
        event_type: String,
        event_text: String
    },
    GameResult {
        home_team_emoji: S,
        home_team_name: S,

        away_team_emoji: S,
        away_team_name: S,

        home_score: u8,
        away_score: u8
    },
    Delivery {
        player_name: S,
        item_emoji: S,
        item: Item,
    },
    AttributeChanges {
        changes: Vec<AttributeChange<S>>
    },
    AttributeEquals {
        equals: Vec<AttributeEqual<S>>
    },
    Enchantment {
        player_name: S,
        item: Item,
        amount: u8,
        attribute: Attribute,
        phrasing: EnchantmentPhrasing
    },
    ROBO {
        player_name: S,
    },
    TakeTheMound {
        to_mound_player: S,
        to_lineup_player: S,
    },
    TakeThePlate {
        to_plate_player: S,
        from_lineup_player: S,
    },
    SwapPlaces {
        player_one: S,
        player_two: S,
    }
}

impl<S> ParsedFeedEventText<S> {
    pub fn is_error(&self) -> bool {
        if let ParsedFeedEventText::ParseError { .. } = self {
            true
        } else {
            false
        }
    }
}

impl<S: Display> Display for ParsedFeedEventText<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.unparse().fmt(f)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributeChange<S> {
    pub player_name: S,
    pub amount: i16,
    pub attribute: Attribute,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributeEqual<S> {
    pub player_name: S,
    pub changing_attribute: Attribute,
    pub value_attribute: Attribute,
}

impl<S: Display> ParsedFeedEventText<S> {
    pub fn unparse(&self) -> String {
        match self {
            ParsedFeedEventText::ParseError { event_text, .. } => event_text.to_string(),
            ParsedFeedEventText::GameResult { home_team_emoji, home_team_name, away_team_emoji, away_team_name, home_score, away_score } => {
                format!("{} {} vs. {} {} - FINAL {}-{}", home_team_emoji, home_team_name, away_team_emoji, away_team_name, home_score, away_score)
            }
            ParsedFeedEventText::Delivery { player_name, item_emoji, item } => {
                format!("{player_name} received a {item_emoji} {item} Delivery.")
            }
            ParsedFeedEventText::AttributeChanges { changes } => {
                changes.iter()
                    .map(|change|  format!("{} gained +{} {}.", change.player_name, change.amount, change.attribute))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            ParsedFeedEventText::AttributeEquals { equals } => {
                equals.iter()
                    .map(|change| format!("{}'s {} became equal to their base {}.", change.player_name, change.changing_attribute, change.value_attribute))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            ParsedFeedEventText::Enchantment { player_name, item, amount, attribute, phrasing } => {
                phrasing.format_event(player_name, *item, *amount, *attribute)
            }
            ParsedFeedEventText::ROBO { player_name } => {
                format!("{player_name} gained the ROBO Modification.")
            }
            ParsedFeedEventText::TakeTheMound { to_mound_player, to_lineup_player } => {
                format!("{to_mound_player} was moved to the mound. {to_lineup_player} was sent to the lineup.")
            }
            ParsedFeedEventText::TakeThePlate { to_plate_player, from_lineup_player } => {
                format!("{to_plate_player} was sent to the plate. {from_lineup_player} was pulled from the lineup.")
            },
            ParsedFeedEventText::SwapPlaces { player_one, player_two } => {
                format!("{player_one} swapped places with {player_two}.")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EnchantmentPhrasing {
    Season1A,
    Season1B
}
impl EnchantmentPhrasing {
    pub fn format_event<S:Display>(&self, player_name: &S, item: Item, amount: u8, attribute: Attribute) -> String {
        match self {
            EnchantmentPhrasing::Season1A => format!("{player_name}'s {item} was enchanted with +{amount} to {attribute}."),
            EnchantmentPhrasing::Season1B => format!("The Item Enchantment was a success! {player_name}'s {item} gained a +{amount} {attribute} bonus.")
        }
    }
}