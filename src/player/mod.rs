use std::collections::HashMap;

pub use serde::{Serialize, Deserialize};

use crate::{enums::{Attribute, Day, EquipmentEffectType, EquipmentRarity, EquipmentSlot, GameStat, Handedness, ItemPrefix, ItemSuffix, ItemType, MaybeRecognized, Position, PositionType, SeasonStatus}, feed_event::FeedEvent, utils::{AddedLater, RemovedLater, ExpectNone}};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Player {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<String>,

    pub augments: u8,
    pub bats: MaybeRecognized<Handedness>,
    pub birthday: MaybeRecognized<Day>,
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub birthseason: AddedLater<Option<u16>>,
    pub durability: f64,
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub equipment: AddedLater<HashMap<MaybeRecognized<EquipmentSlot>, Option<PlayerEquipment>>>,
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub feed: AddedLater<Vec<FeedEvent>>,
    pub first_name: String,
    pub last_name: String,
    pub home: String,


    greater_boon: ExpectNone,
    pub lesser_boon: Option<Boon>,
    pub modifications: Vec<Modification>,

    pub likes: String,
    pub dislikes: String,
    
    pub number: u8,
    pub position: MaybeRecognized<Position>,
    pub position_type: MaybeRecognized<PositionType>,
    pub season_stats: HashMap<String, HashMap<MaybeRecognized<SeasonStatus>, String>>,
    pub stats: HashMap<MaybeRecognized<GameStat>, i32>,

    #[serde(rename = "TeamID")]
    pub team_id: Option<String>,
    pub throws: MaybeRecognized<Handedness>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerEquipment {
    pub effects: Vec<MaybeRecognized<EquipmentEffect>>,
    pub emoji: String,
    #[serde(default, skip_serializing_if = "RemovedLater::skip")]
    pub slot: RemovedLater<Option<MaybeRecognized<EquipmentSlot>>>,
    pub name: MaybeRecognized<ItemType>,
    pub prefix: Option<MaybeRecognized<ItemPrefix>>,
    pub suffix: Option<MaybeRecognized<ItemSuffix>>,
    pub rarity: MaybeRecognized<EquipmentRarity>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct EquipmentEffect {
    pub attribute: MaybeRecognized<Attribute>,
    #[serde(rename = "Type")]
    pub effect_type: MaybeRecognized<EquipmentEffectType>,
    pub value: f64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Modification {
    pub emoji: String,
    pub name: String,
    pub description: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Boon {
    pub emoji: String,
    pub name: String,
    pub description: String,
}
