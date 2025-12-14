use std::fmt::Write;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use itertools::Itertools;

use crate::{enums::{Attribute, FeedEventType, ModificationType}, feed_event::{EmojilessItem, FeedDelivery, FeedEvent, FeedEventParseError, FeedFallingStarOutcome}, time::{Breakpoints, Timestamp}, utils::extra_fields_deserialize};
use crate::enums::Slot;
use crate::feed_event::{AttributeChange, GreaterAugment};
pub use crate::nom_parsing::parse_team_feed_event::parse_team_feed_event;
use crate::nom_parsing::shared::{FeedEventDoorPrize, FeedEventParty, Grow, PositionSwap};
use crate::parsed_event::{EmojiPlayer, EmojiTeam, GrowAttributeChange};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamFeed {
    pub feed: Vec<FeedEvent>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PurifiedOutcome {
    Payment(u32),
    PaymentAndImmunityRemoved(u32),
    NoCorruption,
    None,
}

impl PurifiedOutcome {
    pub fn unparse<S: Display>(&self, player_name: S) -> String {
        match self {
            PurifiedOutcome::Payment(payment) => format!("{player_name} was Purified of 🫀 Corruption and earned {payment} 🪙."),
            PurifiedOutcome::PaymentAndImmunityRemoved(payment) => format!("{player_name} was Purified of 🌹 Efflorescence, earned {payment} 🪙, and gained 🦠 Immunity."),
            PurifiedOutcome::NoCorruption => format!("{player_name} was Purified of 🫀 Corruption. {player_name} had no Corruption to remove."),
            PurifiedOutcome::None => format!("{player_name} was Purified of 🫀 Corruption."),
        }
    }
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
        player: Option<EmojiPlayer<S>>,
        earned_coins: u32,
    },
    Party {
        party: FeedEventParty<S>,
    },
    DoorPrize {
        prize: FeedEventDoorPrize<S>,
    },
    Prosperous {
        team: EmojiTeam<S>,
        income: u32
    },
    DonatedToLottery {
        team_name: S,
        amount: u32,
        league_name: S,
    },
    WonLottery {
        amount: u32,
        league_name: S,
    },
    Enchantment {
        team_name: S,
        item: EmojilessItem,
        amount: u8,
        attribute: Attribute,
        enchant_two: Option<(u8, Attribute)>,
        compensatory: bool
    },
    AttributeChanges {
        changes: Vec<AttributeChange<S>>
    },
    MassAttributeEquals {
        players: Vec<(Option<Slot>, S)>,
        changing_attribute: Attribute,
        value_attribute: Attribute,
    },
    TakeTheMound {
        to_mound_team: S,
        to_lineup_team: S,
    },
    TakeThePlate {
        to_plate_team: S,
        from_lineup_team: S,
    },
    SwapPlaces {
        team_one: S,
        team_two: S,
    },
    Recomposed {
        previous: S,
        new: S
    },
    Modification {
        team_name: S,
        lost_modification: Option<ModificationType>,
        modification: ModificationType
    },
    FallingStarOutcome {
        player_name: S,
        outcome: FeedFallingStarOutcome
    },
    CorruptedByWither {
        player_name: S,
    },
    Purified {
        player_name: S,
        outcome: PurifiedOutcome,
    },
    NameChanged,
    PlayerMoved {
        team_emoji: S,
        player_name: S,
    },
    PlayerRelegated {
        player_name: S,
    },
    PlayerPositionsSwapped {
        swap: PositionSwap<S>,
    },
    PlayerContained {
        contained_player_name: S,
        container_player_name: S,
    },
    PlayerGrow {
        grow: Grow<S>
    },
    Callup {
        lesser_league_team: EmojiTeam<S>,
        greater_league_team: EmojiTeam<S>,
        slot: Slot,
        promoted_player_name: S,
        demoted_player_name: S,
    },
    GreaterAugment {
        team: EmojiTeam<S>,
        greater_augment: GreaterAugment,
    },
    PlayerGrewInEfflorescence {
        player_name: S,
        growths: [GrowAttributeChange; 2],
    },
    PlayerEffloresce {
        player_name: S,
    },
    ClaimedLinealBelt {
        team: EmojiTeam<S>,
        old_belt_holder_team: EmojiTeam<S>,
    },
    LostLinealBelt {
        team: EmojiTeam<S>,
        new_belt_holder_team: EmojiTeam<S>,
    },
    // TODO Delete any of these that are still unused when parsing is up to date

    Released {
        team: S
    },
    Retirement {
        previous: S,
        new: Option<S>
    },
}

impl<S: Display> ParsedTeamFeedEventText<S> {
    pub fn unparse(&self, event: &FeedEvent) -> String {
        match self {
            ParsedTeamFeedEventText::ParseError { error: _, text } => text.to_string(),
            ParsedTeamFeedEventText::GameResult { home_team, away_team, home_score, away_score } => {
                format!("{} vs. {} - FINAL {}-{}", away_team, home_team, away_score, home_score)
            }
            ParsedTeamFeedEventText::Delivery { delivery } => delivery.unparse(event, "Delivery"),
            ParsedTeamFeedEventText::Shipment { delivery } => delivery.unparse(event, "Shipment"),
            ParsedTeamFeedEventText::SpecialDelivery { delivery } => delivery.unparse(event, "Special Delivery"),
            ParsedTeamFeedEventText::PhotoContest { player, earned_coins } => {
                match player {
                    None => format!("Earned {earned_coins} 🪙 in the Photo Contest."),
                    Some(pl) => format!("{} {} won {earned_coins} 🪙 in a Photo Contest.", pl.emoji, pl.name),
                }
            },
            ParsedTeamFeedEventText::Party { party } => {
                format!("{party}")
            }
            ParsedTeamFeedEventText::DoorPrize { prize } => {
                format!("{prize}")
            }
            ParsedTeamFeedEventText::Prosperous { team, income} => {
                let verb = if Breakpoints::Season5TenseChange.before(event.season as u32, event.day.as_ref().ok().copied(), None) {
                    "earned"
                } else {
                    "earn"
                };

                format!("{team} are Prosperous! They {verb} {income} 🪙.")
            },
            ParsedTeamFeedEventText::DonatedToLottery { team_name, amount, league_name } => {
                format!("The {team_name} donated {amount} 🪙 to the {league_name} Lottery.")
            }
            ParsedTeamFeedEventText::WonLottery { amount, league_name } => {
                format!("Won {amount} 🪙 from the {league_name} Lottery!")
            }
            ParsedTeamFeedEventText::FallingStarOutcome { player_name, outcome } => {
                outcome.unparse(event, player_name)
            }
            ParsedTeamFeedEventText::AttributeChanges { changes } => {
                changes
                    .iter()
                    .map(|change| format!("{} gained +{} {}.", change.player_name, change.amount, change.attribute))
                    .join(" ")
            },
            ParsedTeamFeedEventText::MassAttributeEquals { players, changing_attribute, value_attribute } => {
                if Breakpoints::Season3.after(event.season as u32, event.day.as_ref().copied().ok(), None) {
                    let intro = format!("Batters' {changing_attribute} was set to their {value_attribute}. Lineup:");
                    let lineup = players.iter()
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
                    players.iter()
                        .map(|(_, p)| f(p, changing_attribute, value_attribute))
                        .collect::<Vec<_>>()
                        .join(" ")
                }
                    },
            ParsedTeamFeedEventText::Recomposed { previous, new } => {
                        if event.timestamp > Timestamp::Season3RecomposeChange.timestamp() {
                            format!("{previous} was Recomposed into {new}.")
                        } else {
                            format!("{previous} was Recomposed using {new}.")
                        }
                    },
            ParsedTeamFeedEventText::TakeTheMound { to_mound_team, to_lineup_team } => format!("{to_mound_team} was moved to the mound. {to_lineup_team} was sent to the lineup."),
            ParsedTeamFeedEventText::TakeThePlate { to_plate_team, from_lineup_team } => format!("{to_plate_team} was sent to the plate. {from_lineup_team} was pulled from the lineup."),
            ParsedTeamFeedEventText::SwapPlaces { team_one, team_two } => format!("{team_one} swapped places with {team_two}."),
            ParsedTeamFeedEventText::Enchantment { team_name, item, amount, attribute, enchant_two, compensatory } => {
                        if event.before(Breakpoints::Season1EnchantmentChange) {
                            if enchant_two.is_some() {
                                tracing::error!("Season 1 enchantment had two enchants");
                            }
                            if *compensatory {
                                tracing::error!("Season 1 enchantment was compensatory")
                            }
                            format!("{team_name}'s {item} was enchanted with +{amount} to {attribute}.")
                        } else if event.before(Breakpoints::season(2)) {
                            if enchant_two.is_some() {
                                tracing::error!("Season 1 enchantment had two enchants");
                            }
                            if *compensatory {
                                tracing::error!("Season 1 enchantment was compensatory")
                            }
                            format!("The Item Enchantment was a success! {team_name}'s {item} gained a +{amount} {attribute} bonus.")
                        } else {
                            let enchant_type = compensatory.then_some("Compensatory").unwrap_or("Item");
                            match enchant_two {
                                Some((amount_two, attribute_two)) => format!("The {enchant_type} Enchantment was a success! {team_name}'s {item} was enchanted with +{amount} {attribute} and +{amount_two} {attribute_two}."),
                                None =>  format!("The {enchant_type} Enchantment was a success! {team_name}'s {item} gained a +{amount} {attribute} bonus.")
                            }
                        }
                    },
            ParsedTeamFeedEventText::Released { team } => format!("Released by the {team}."),
            ParsedTeamFeedEventText::Modification { team_name, lost_modification, modification } => {
                match lost_modification {
                    Some(lost_modification) => format!("{team_name} lost the {lost_modification} Modification. {team_name} gained the {modification} Modification."),
                    None => format!("{team_name} gained the {modification} Modification.")
                }
            },
            ParsedTeamFeedEventText::Retirement { previous, new } => {
                let new = new.as_ref().map(|new| format!(" {new} was called up to take their place.")).unwrap_or_default();
                let emoji = (matches!(event.event_type, Ok(FeedEventType::Game))).then_some("😇 ").unwrap_or_default();
                format!("{emoji}{previous} retired from MMOLB!{new}")
            }
            ParsedTeamFeedEventText::CorruptedByWither { player_name } => {
                format!("{player_name} was Corrupted by the 🥀 Wither.")
            }
            ParsedTeamFeedEventText::Purified { player_name, outcome } => {
                outcome.unparse(player_name)
            }
            ParsedTeamFeedEventText::NameChanged => {
                "The team's name was reset in accordance with site policy.".to_string()
            },
            ParsedTeamFeedEventText::PlayerMoved { team_emoji, player_name } => {
                format!("{team_emoji} {player_name} was moved to the Bench.")
            },
            ParsedTeamFeedEventText::PlayerRelegated { player_name } => {
                format!("🧳 {player_name} was relegated to the Even Lesser League.")
            },
            ParsedTeamFeedEventText::PlayerPositionsSwapped { swap } => {
                format!("{swap}")
            },
            ParsedTeamFeedEventText::PlayerContained { contained_player_name, container_player_name } => {
                format!(
                    "{contained_player_name} was contained by {container_player_name} during the \
                    🥀 Wither.",
                )
            },
            ParsedTeamFeedEventText::PlayerGrow { grow } => {
                format!("{grow}")
            },
            ParsedTeamFeedEventText::Callup { lesser_league_team, greater_league_team, slot, promoted_player_name, demoted_player_name } => {
                format!(
                    "{lesser_league_team} {slot} {promoted_player_name} was called up to replace \
                    {greater_league_team} {slot} {demoted_player_name}. {demoted_player_name} \
                    joined the {}.", lesser_league_team.name
                )
            }
            ParsedTeamFeedEventText::GreaterAugment { team, greater_augment } => {
                format!("{team} selected {}", match greater_augment {
                    GreaterAugment::StartSmall => "Start Small, improving their Starting Pitchers.",
                    GreaterAugment::Headliners => "Headliners, improving the three Batters at the top of their Lineup.",
                    GreaterAugment::Plating => "Reinforced Plating, granting their Players +10 to all Defense Attributes.",
                    GreaterAugment::LuckyDelivery => "TODO Insert the lucky delivery text here",
                })
            }
            ParsedTeamFeedEventText::PlayerGrewInEfflorescence { player_name, growths: [grow_1, grow_2] } => {
                format!("{player_name} grew in the 🌹 Efflorescence: {grow_1}, {grow_2}.")
            }
            ParsedTeamFeedEventText::PlayerEffloresce { player_name} => {
                format!("{player_name} is Efflorescing and sheds their Corruption!")
            }
            ParsedTeamFeedEventText::ClaimedLinealBelt { team, old_belt_holder_team} => {
                format!("{team} claimed the Lineal Belt from {old_belt_holder_team}!")
            }
            ParsedTeamFeedEventText::LostLinealBelt { team, new_belt_holder_team } => {
                format!("{team} lost the Lineal Belt to {new_belt_holder_team}.")
            }
        }
    }
}
