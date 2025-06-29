use std::{fmt::Display, ops::{Deref, DerefMut}};

use serde::{Deserialize, Serialize};

use crate::{enums::{Attribute, FeedEventType, ItemPrefix, ItemSuffix, ItemType}, nom_parsing::parse_feed_event, parsed_event::Item};

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
        home_team_emoji: S,
        home_team_name: S,

        away_team_emoji: S,
        away_team_name: S,

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
        phrasing: AttributeEqualsPhrasing
    },
    S1Enchantment {
        player_name: S,
        item: EmojilessItem,
        amount: u8,
        attribute: Attribute,
        phrasing: S1EnchantmentPhrasing,
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
    pub fn unparse(&self) -> String {
        match self {
            ParsedFeedEventText::ParseError { event_text, .. } => event_text.to_string(),
            ParsedFeedEventText::GameResult { home_team_emoji, home_team_name, away_team_emoji, away_team_name, home_score, away_score } => {
                format!("{} {} vs. {} {} - FINAL {}-{}", home_team_emoji, home_team_name, away_team_emoji, away_team_name, home_score, away_score)
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
            ParsedFeedEventText::AttributeEquals { equals, phrasing } => {
                let current = match phrasing {
                    AttributeEqualsPhrasing::Season1 => "",
                    AttributeEqualsPhrasing::Season2 => "current "
                };
                equals.iter()
                    .map(|change| format!("{}'s {} became equal to their {current}base {}.", change.player_name, change.changing_attribute, change.value_attribute))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute, phrasing } => {
                phrasing.format_event(player_name, *item, *amount, *attribute)
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
            }
        }
    }
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum AttributeEqualsPhrasing {
    Season1,
    Season2
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum S1EnchantmentPhrasing {
    Season1A,
    Season1B
}
impl S1EnchantmentPhrasing {
    pub fn format_event<S:Display>(&self, player_name: &S, item: EmojilessItem, amount: u8, attribute: Attribute) -> String {
        match self {
            S1EnchantmentPhrasing::Season1A => format!("{player_name}'s {item} was enchanted with +{amount} to {attribute}."),
            S1EnchantmentPhrasing::Season1B => format!("The Item Enchantment was a success! {player_name}'s {item} gained a +{amount} {attribute} bonus.")
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
