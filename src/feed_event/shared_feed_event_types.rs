use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::enums::{BenchSlot, CelestialEnergyTier, FullSlotLabel, ModificationType};
use crate::{
    enums::{Attribute, FeedEventType, ItemName, ItemPrefix, ItemSuffix},
    feed_event::FeedEvent,
    game_time::Breakpoints,
    parsed_event::{GrowAttributeChange, Item, Prize},
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
pub struct PositionSwap<S> {
    pub first_player_name: S,
    pub first_player_new_slot: FullSlotLabel,
    pub second_player_name: S,
    pub second_player_new_slot: FullSlotLabel,
}

impl<S: Display> Display for PositionSwap<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let PositionSwap {
            first_player_name,
            first_player_new_slot,
            second_player_name,
            second_player_new_slot,
        } = self;

        write!(
            f,
            "{first_player_name} and {second_player_name} swapped positions: \
            {first_player_name} moved to {first_player_new_slot}, \
            {second_player_name} moved to {second_player_new_slot}."
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedEventParty<S> {
    pub player_name: S,
    pub amount_gained: u8,
    pub attribute: Attribute,
    // As of this writing, the Prolific boon is the only way to not lose durability
    pub durability_lost: Option<u8>,
}

impl<S: Display> Display for FeedEventParty<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is Partying! {} gained +{} {} and ",
            self.player_name, self.player_name, self.amount_gained, self.attribute,
        )?;

        match self.durability_lost {
            None => write!(f, "their Prolific Greater Boon resisted Durability loss."),
            Some(durability_lost) => write!(f, "lost {durability_lost} Durability."),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GainedImmovable {
    No,
    Yes,
    YesReplacing(ModificationType),
    BenchPlayerImmune,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Grow<S> {
    pub player_name: S,
    pub attribute_changes: [GrowAttributeChange; 3],
    pub immovable_granted: GainedImmovable,
}

impl<S: Display> Display for Grow<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}'s Corruption grew: ", self.player_name)?;
        for (i, change) in self.attribute_changes.iter().enumerate() {
            let prefix = if i == 0 { "" } else { ", " };
            // Don't use GrowAttributeChange's Display implementation because it's the
            // wrong number of decimal places
            write!(f, "{prefix}{:+.1} {}", change.amount, change.attribute)?;
        }
        match &self.immovable_granted {
            GainedImmovable::No => write!(f, "."),
            GainedImmovable::Yes => write!(
                f,
                ". {} gained the Immovable Greater Boon.",
                self.player_name
            ),
            GainedImmovable::YesReplacing(replaced) => write!(
                f,
                ". {} gained the Immovable Greater Boon, replacing {replaced}.",
                self.player_name
            ),
            GainedImmovable::BenchPlayerImmune => write!(
                f,
                ". {} could not gain Immovable while on the Bench.",
                self.player_name
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeedEventDoorPrize<S> {
    pub player_name: S,
    pub prize: Prize<S>,
}

impl<S: Display> Display for FeedEventDoorPrize<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let punct = match &self.prize {
            Prize::Items(i) if i.iter().any(|prize| !prize.equip.is_none()) => "!",
            _ => ":",
        };
        write!(
            f,
            "{} won a Door Prize{punct} {}.",
            self.player_name,
            self.prize.unparse()
        )
    }
}
