use std::{fmt::Display, ops::{Deref, DerefMut}};

use serde::{Deserialize, Serialize};

use crate::{enums::{Attribute, FeedEventType, ItemPrefix, ItemSuffix, ItemType}, feed_event::FeedEvent, nom_parsing::parse_feed_event, parsed_event::{EmojiTeam, Item}, time::Breakpoints};

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

impl PartialEq<String> for FeedEventText {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}
impl Display for FeedEventText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParsedFeedEventText<S> {
    ParseError {
        event_type: String,
        event_text: String
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
    AttributeChanges {
        changes: Vec<AttributeChange<S>>
    },
    AttributeEquals {
        equals: Vec<AttributeEqual<S>>,
    },
    S1Enchantment {
        player_name: S,
        item: EmojilessItem,
        amount: u8,
        attribute: Attribute,
    },
    S2Enchantment {
        player_name: S,
        item: EmojilessItem,
        amount: u8,
        attribute: Attribute,
        enchant_two: Option<(u8, Attribute)>,
        compensatory: bool
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
    },
    HitByFallingStar {
        player: S
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AttributeChange<S> {
    pub player_name: S,
    pub amount: i16,
    pub attribute: Attribute,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AttributeEqual<S> {
    pub player_name: S,
    pub changing_attribute: Attribute,
    pub value_attribute: Attribute,
}

impl<S: Display> ParsedFeedEventText<S> {
    pub fn unparse(&self, event: &FeedEvent) -> String {
        match self {
            ParsedFeedEventText::ParseError { event_text, .. } => event_text.to_string(),
            ParsedFeedEventText::GameResult { home_team, away_team, home_score, away_score } => {
                format!("{} vs. {} - FINAL {}-{}", home_team, away_team, home_score, away_score)
            }
            ParsedFeedEventText::Delivery { delivery } => {
                delivery.unparse("Delivery")
            }
            ParsedFeedEventText::SpecialDelivery { delivery } => {
                delivery.unparse("Special Delivery")
            }
            ParsedFeedEventText::Shipment { delivery } => {
                delivery.unparse("Shipment")
            }
            ParsedFeedEventText::AttributeChanges { changes } => {
                changes.iter()
                    .map(|change|  format!("{} gained +{} {}.", change.player_name, change.amount, change.attribute))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            ParsedFeedEventText::AttributeEquals { equals } => {
                let current = Breakpoints::S1AttributeEqualChange.after(event.season as u32, &event.day, None).then_some("current ").unwrap_or_default();
                equals.iter()
                    .map(|change| format!("{}'s {} became equal to their {current}base {}.", change.player_name, change.changing_attribute, change.value_attribute))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute } => {
                if Breakpoints::Season1EnchantmentChange.before(event.season as u32, &event.day, None) {
                    format!("{player_name}'s {item} was enchanted with +{amount} to {attribute}.")
                } else {
                    format!("The Item Enchantment was a success! {player_name}'s {item} gained a +{amount} {attribute} bonus.")
                }
            }
            ParsedFeedEventText::S2Enchantment { player_name, item, amount, attribute, enchant_two, compensatory } => {
                let enchant_type = compensatory.then_some("Compensatory").unwrap_or("Item");
                match enchant_two {
                    Some((amount_two, attribute_two)) => format!("The {enchant_type} Enchantment was a success! {player_name}'s {item} was enchanted with +{amount} {attribute} and +{amount_two} {attribute_two}."),
                    None =>  format!("The {enchant_type} Enchantment was a success! {player_name}'s {item} gained a +{amount} {attribute} bonus.")
                }
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
            },
            ParsedFeedEventText::HitByFallingStar { player } => {
                format!("{player} was hit by a Falling Star!")
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedDelivery<S> {
    pub player: S,
    pub item: Item<S>,
    pub discarded: Option<Item<S>>
}
impl<S: Display> FeedDelivery<S> {
    pub fn unparse(&self, delivery_label: &str) -> String {
        let FeedDelivery { player, item, discarded} = self;

        let discarded = match discarded {
            Some(discarded) => format!(" They discarded their {discarded}."),
            None => String::new(),
        };


        format!("{player} received a {item} {delivery_label}.{discarded}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EmojilessItem {
    pub prefix: Option<ItemPrefix>,
    pub item: ItemType,
    pub suffix: Option<ItemSuffix>,
}
impl Display for EmojilessItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let EmojilessItem { prefix, item, suffix } = self;
        let prefix = match prefix {
            Some(prefix) => format!("{prefix} "),
            None => String::new()
        };
        let suffix = match suffix {
            Some(suffix) => format!(" of {suffix}"),
            None => String::new()
        };

        write!(f, "{prefix}{item}{suffix}")
    }
}
