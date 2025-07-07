use std::collections::HashMap;

pub use serde::{Serialize, Deserialize};

use crate::{enums::{Attribute, Day, EquipmentEffectType, EquipmentRarity, EquipmentSlot, GameStat, Handedness, ItemPrefix, ItemSuffix, ItemType, MaybeRecognized, Position, PositionType, SeasonStatus}, feed_event::FeedEvent, utils::{AddedLater, ExpectNone}};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Player {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<String>,

    pub augments: u8,
    pub bats: MaybeRecognized<Handedness>,
    pub birthday: MaybeRecognized<Day>,
    /// Not present on old, deleted players
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub birthseason: AddedLater<u16>,
    pub durability: f64,
    /// Not present on old, deleted players
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub equipment: AddedLater<PlayerEquipmentMap>,
    /// Not present on old, deleted players
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

/// A player's equipment field can be described by `HashMap<MaybeRecognized<EquipmentSlot>, Option<PlayerEquipment>>`
/// 
/// This wrapper is accessed more like `HashMap<MaybeRecognized<EquipmentSlot>, PlayerEquipment>`, and can be accessed through 
/// an `EquipmentSlot` on its own as well as an `&MaybeRecognized<EquipmentSlot>`.
/// 
/// ```
/// use std::collections::HashMap;
/// use mmolb_parsing::player::{PlayerEquipmentMap, PlayerEquipment};
/// use mmolb_parsing::enums::{MaybeRecognized, EquipmentSlot};
///  
/// let map = PlayerEquipmentMap::default();
/// map.get(EquipmentSlot::Head);
/// map.get(&MaybeRecognized::Recognized(EquipmentSlot::Head));
/// map.get(&MaybeRecognized::NotRecognized(serde_json::Value::String("New Slot".to_string())));
/// 
/// let a: HashMap<MaybeRecognized<EquipmentSlot>, PlayerEquipment> = map.clone().into();
/// let b: HashMap<MaybeRecognized<EquipmentSlot>, Option<PlayerEquipment>> = map.clone().into();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerEquipmentMap {
    #[serde(flatten)]
    fields: HashMap<MaybeRecognized<EquipmentSlot>, Option<PlayerEquipment>>,
}

impl PlayerEquipmentMap {
    pub fn get<T>(&self, index: T) -> Option<&PlayerEquipment> where Self: _GetHelper<T, Output = PlayerEquipment> {
        self._get(index)
    }
    pub fn get_mut<T>(&mut self, index: T) -> Option<&mut PlayerEquipment> where Self: _GetHelper<T, Output = PlayerEquipment> {
        self._get_mut(index)
    }
}

impl Into<HashMap<MaybeRecognized<EquipmentSlot>, PlayerEquipment>> for PlayerEquipmentMap {
    fn into(self) -> HashMap<MaybeRecognized<EquipmentSlot>, PlayerEquipment> {
        self.fields.into_iter()
            .flat_map(|(slot, equipment)| equipment.and_then(|e| Some((slot, e))))
            .collect()
    }
}

impl Into<HashMap<MaybeRecognized<EquipmentSlot>, Option<PlayerEquipment>>> for PlayerEquipmentMap {
    fn into(self) -> HashMap<MaybeRecognized<EquipmentSlot>, Option<PlayerEquipment>> {
        self.fields
    }
}

pub trait _GetHelper<Index> {
    type Output;
    fn _get(&self, index: Index) -> Option<&Self::Output>;
    fn _get_mut(&mut self, index: Index) -> Option<&mut Self::Output>;
}


impl _GetHelper<EquipmentSlot> for PlayerEquipmentMap {
    type Output = PlayerEquipment;
    fn _get(&self, index: EquipmentSlot) -> Option<&Self::Output> {
        self._get(&MaybeRecognized::Recognized(index))
    }
    fn _get_mut(&mut self, index: EquipmentSlot) -> Option<&mut Self::Output> {
        self._get_mut(&MaybeRecognized::Recognized(index))
    }
}

impl _GetHelper<&MaybeRecognized<EquipmentSlot>> for PlayerEquipmentMap {
    type Output = PlayerEquipment;

    fn _get(&self, index: &MaybeRecognized<EquipmentSlot>) -> Option<&Self::Output> {
        self.fields.get(index).map(Option::as_ref).flatten()
    }
    fn _get_mut(&mut self, index: &MaybeRecognized<EquipmentSlot>) -> Option<&mut Self::Output> {
        self.fields.get_mut(index).map(Option::as_mut).flatten()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerEquipment {
    pub effects: Vec<MaybeRecognized<EquipmentEffect>>,
    pub emoji: String,
    /// Removed in the current version of the API
    #[serde(default, skip_serializing_if = "Option::is_none")]
    slot: Option<MaybeRecognized<EquipmentSlot>>,
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

#[cfg(test)]
mod test {
    use std::path::Path;

    use tracing_test::traced_test;

    use crate::{player::Player, utils::assert_round_trip};


    #[test]
    #[traced_test]
    fn player_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        assert_round_trip::<Player>(Path::new("test_data/s2_player.json"))?;
        assert!(!logs_contain("not recognized"));
        Ok(())
    }
}