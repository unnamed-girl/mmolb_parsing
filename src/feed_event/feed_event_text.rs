use std::fmt::Display;

use thiserror::Error;
use serde::{Serialize, Deserialize};

use crate::{enums::{Attribute, CelestialEnergyTier, FeedEventSource, FeedEventType, ItemName, ItemPrefix, ItemSuffix, ModificationType, Slot}, feed_event::FeedEvent, parsed_event::{EmojiTeam, Item}, time::{Breakpoints, Timestamp}, NotRecognized};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Error)]
pub enum FeedEventParseError {
    #[error("feed event type {} not recognized", .0.0)]
    EventTypeNotRecognized(#[source] NotRecognized),
    #[error("failed parsing {event_type} feed event \"{text}\"")]
    FailedParsingText {
        event_type: FeedEventType,
        text: String
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParsedFeedEventText<S> {
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
    AttributeChanges {
        changes: Vec<AttributeChange<S>>
    },
    SingleAttributeEquals {
        player_name: S,
        changing_attribute: Attribute,
        value_attribute: Attribute,
    },
    MassAttributeEquals {
        players: Vec<(Option<Slot>, S)>,
        changing_attribute: Attribute,
        value_attribute: Attribute,
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
    Prosperous {
        team: EmojiTeam<S>,
        income: u8
    },
    Recomposed {
        previous: S,
        new: S
    },
    Modification {
        player_name: S,
        modification: ModificationType
    },
    Retirement {
        previous: S,
        new: Option<S>
    },
    InjuredByFallingStar {
        player: S
    },
    InfusedByFallingStar {
        player: S,
        infusion_tier: CelestialEnergyTier
    },
    Released {
        team: S
    }
}

impl<S: Display> ParsedFeedEventText<S> {
    pub fn unparse(&self, event: &FeedEvent, source: FeedEventSource) -> String {
        match self {
            ParsedFeedEventText::ParseError { text, .. } => text.to_string(),
            ParsedFeedEventText::GameResult { home_team, away_team, home_score, away_score } => {
                        format!("{} vs. {} - FINAL {}-{}", away_team, home_team, away_score, home_score)
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
            ParsedFeedEventText::SingleAttributeEquals { player_name, changing_attribute, value_attribute } => {
                        if Breakpoints::Season3.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                            format!("{}'s {} was set to their {}.", player_name, changing_attribute, value_attribute)
                        } else if Breakpoints::S1AttributeEqualChange.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                            format!("{}'s {} became equal to their current base {}.", player_name, changing_attribute, value_attribute)
                        } else if FeedEventSource::Player == source {
                            format!("{}'s {} was set to their {}.", player_name, changing_attribute, value_attribute)
                        } else {
                            format!("{}'s {} became equal to their base {}.", player_name, changing_attribute, value_attribute)
                        }
                    },
            ParsedFeedEventText::MassAttributeEquals { players, changing_attribute, value_attribute } => {
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
                                } else if FeedEventSource::Player == source {
                                    format!("{}'s {} was set to their {}.", player_name, changing_attribute, value_attribute)
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
            ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute } => {
                        if Breakpoints::Season1EnchantmentChange.before(event.season as u32, event.day.as_ref().copied().ok(), None) {
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
            ParsedFeedEventText::Modification { player_name, modification } => {
                        format!("{player_name} gained the {modification} Modification.")
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
            ParsedFeedEventText::Prosperous { team, income } => {
                        format!("{team} are Prosperous! They earned {income} 🪙.")
                    },
            ParsedFeedEventText::Recomposed { previous, new } => {
                if event.timestamp > Timestamp::Season3RecomposeChange.timestamp() {
                    format!("{previous} was Recomposed into {new}.")
                } else {
                    format!("{previous} was Recomposed using {new}.")
                }
            },
            ParsedFeedEventText::Retirement { previous, new } => {
                let new = new.as_ref().map(|new| format!(" {new} was called up to take their place.")).unwrap_or_default();
                format!("😇 {previous} retired from MMOLB!{new}")
            },
            ParsedFeedEventText::InjuredByFallingStar { player } => {
                if event.after(Breakpoints::EternalBattle) {
                    format!("{player} was injured by the extreme force of the impact!")
                } else {
                    format!("{player} was hit by a Falling Star!")
                }
            }
            ParsedFeedEventText::InfusedByFallingStar { player, infusion_tier } => {
                format!("{player} {infusion_tier}")
            },
            ParsedFeedEventText::Released { team } => {
                format!("Released by the {team}.")
            }
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
    pub item: ItemName,
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
            Some(suffix) => format!(" {suffix}"),
            None => String::new()
        };

        write!(f, "{prefix}{item}{suffix}")
    }
}
