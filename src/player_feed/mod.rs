use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{enums::{Attribute, FeedEventType, ModificationType}, feed_event::{EmojilessItem, FeedDelivery, FeedEvent, FeedEventParseError, FeedFallingStarOutcome}, time::{Breakpoints, Timestamp}, utils::extra_fields_deserialize};
use crate::feed_event::GreaterAugment;
pub use crate::nom_parsing::parse_player_feed_event::parse_player_feed_event;
use crate::nom_parsing::shared::{FeedEventDoorPrize, FeedEventParty, Grow, PositionSwap};
use crate::team_feed::{ParsedTeamFeedEventText, PurifiedOutcome};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerFeed {
    pub feed: Vec<FeedEvent>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ParsedPlayerFeedEventText<S> {
    ParseError {
        error: FeedEventParseError,
        text: S
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
    DoorPrize {
        prize: FeedEventDoorPrize<S>,
    },
    AttributeChanges {
        player_name: S,
        amount: i16,
        attribute: Attribute,
    },
    AttributeEquals {
        player_name: S,
        changing_attribute: Attribute,
        value_attribute: Attribute,
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

    Enchantment {
        player_name: S,
        item: EmojilessItem,
        amount: u8,
        attribute: Attribute,
        enchant_two: Option<(u8, Attribute)>,
        compensatory: bool
    },

    FallingStarOutcome {
        player_name: S,
        outcome: FeedFallingStarOutcome
    },
    Recomposed {
        previous: S,
        new: S
    },
    Released {
        team: S
    },
    Retirement {
        previous: S,
        new: Option<S>
    },
    Modification {
        player_name: S,
        lost_modification: Option<ModificationType>,
        modification: ModificationType
    },
    SeasonalDurabilityLoss {
        player_name: S,
        durability_lost: u32,
        season: u32,
    },
    CorruptedByWither {
        player_name: S,
    },
    Purified {
        player_name: S,
        outcome: PurifiedOutcome,
    },
    Party {
        party: FeedEventParty<S>,
    },
    PlayerContained {
        contained_player_name: S,
        container_player_name: S,
    },
    PlayerPositionsSwapped {
        swap: PositionSwap<S>,
    },
    PlayerGrow {
        grow: Grow<S>
    },
    GreaterAugment {
        player_name: S,
        greater_augment: GreaterAugment,
    }
}

impl<S: Display> ParsedPlayerFeedEventText<S> {
    pub fn unparse(&self, event: &FeedEvent) -> String {
        match self {
            ParsedPlayerFeedEventText::ParseError { error: _, text } => text.to_string(),
            ParsedPlayerFeedEventText::Delivery { delivery } => delivery.unparse(event, "Delivery"),
            ParsedPlayerFeedEventText::SpecialDelivery { delivery } => delivery.unparse(event, "Special Delivery"),
            ParsedPlayerFeedEventText::Shipment { delivery } => delivery.unparse(event, "Shipment"),
            ParsedPlayerFeedEventText::DoorPrize { prize } => prize.to_string(),
            ParsedPlayerFeedEventText::FallingStarOutcome { player_name, outcome } => {
                outcome.unparse(event, player_name)
            }
            ParsedPlayerFeedEventText::AttributeChanges { player_name, amount, attribute } => format!("{player_name} gained +{amount} {attribute}."),
            ParsedPlayerFeedEventText::AttributeEquals { player_name, changing_attribute, value_attribute } => {
                        if Breakpoints::Season3.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                            format!("{}'s {} was set to their {}.", player_name, changing_attribute, value_attribute)
                        } else if Breakpoints::S1AttributeEqualChange.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                            format!("{}'s {} became equal to their current base {}.", player_name, changing_attribute, value_attribute)
                        } else {
                            format!("{}'s {} was set to their {}.", player_name, changing_attribute, value_attribute)
                        }
                    },
            ParsedPlayerFeedEventText::Recomposed { previous, new } => {
                        if event.timestamp > Timestamp::Season3RecomposeChange.timestamp() {
                            format!("{previous} was Recomposed into {new}.")
                        } else {
                            format!("{previous} was Recomposed using {new}.")
                        }
                    },
            ParsedPlayerFeedEventText::TakeTheMound { to_mound_player, to_lineup_player } => format!("{to_mound_player} was moved to the mound. {to_lineup_player} was sent to the lineup."),
            ParsedPlayerFeedEventText::TakeThePlate { to_plate_player, from_lineup_player } => format!("{to_plate_player} was sent to the plate. {from_lineup_player} was pulled from the lineup."),
            ParsedPlayerFeedEventText::SwapPlaces { player_one, player_two } => format!("{player_one} swapped places with {player_two}."),
            ParsedPlayerFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two, compensatory } => {
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
            ParsedPlayerFeedEventText::Released { team } => format!("Released by the {team}."),
            ParsedPlayerFeedEventText::Modification { player_name, lost_modification, modification } => {
                match lost_modification {
                    Some(lost_modification) => format!("{player_name} lost the {lost_modification} Modification. {player_name} gained the {modification} Modification."),
                    None => format!("{player_name} gained the {modification} Modification.")
                }
            },
            ParsedPlayerFeedEventText::Retirement { previous, new } => {
                let new = new.as_ref().map(|new| format!(" {new} was called up to take their place.")).unwrap_or_default();
                let emoji = (matches!(event.event_type, Ok(FeedEventType::Game))).then_some("😇 ").unwrap_or_default();
                format!("{emoji}{previous} retired from MMOLB!{new}")
            }
            ParsedPlayerFeedEventText::SeasonalDurabilityLoss { player_name, durability_lost, season } => {
                format!("{player_name} lost {durability_lost} durability for playing in Season {season}.")
            }
            ParsedPlayerFeedEventText::CorruptedByWither { player_name } => {
                format!("{player_name} was Corrupted by the 🥀 Wither.")
            }
            ParsedPlayerFeedEventText::Purified { player_name, outcome } => {
                outcome.unparse(player_name)
            }
            ParsedPlayerFeedEventText::Party { party } => {
                format!("{party}")
            }
            ParsedPlayerFeedEventText::PlayerContained { contained_player_name, container_player_name } => {
                // TODO Dedup with player feed
                format!(
                    "{contained_player_name} was contained by {container_player_name} during the \
                    🥀 Wither.",
                )
            }
            ParsedPlayerFeedEventText::PlayerPositionsSwapped { swap } => {
                format!("{swap}")
            }
            ParsedPlayerFeedEventText::PlayerGrow { grow } => {
                format!("{grow}")
            }
            ParsedPlayerFeedEventText::GreaterAugment { player_name, greater_augment } => {
                format!("{player_name} {}.", match greater_augment {
                    GreaterAugment::Headliners => "gained +10 to all Defense Attributes",
                    GreaterAugment::StartSmall => "gained +10 to all Defense Attributes",
                    GreaterAugment::Plating => "gained +10 to all Defense Attributes",
                    GreaterAugment::LuckyDelivery => "gained +10 to all Defense Attributes",
                })
            }
        }
    }
}
