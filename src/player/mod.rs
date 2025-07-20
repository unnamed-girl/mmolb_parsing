use std::collections::HashMap;

pub use serde::{Serialize, Deserialize};
use serde_with::serde_as;

use crate::{enums::{Attribute, Day, EquipmentEffectType, EquipmentRarity, EquipmentSlot, GameStat, Handedness, ItemPrefix, ItemSuffix, ItemType, Position, PositionType, SeasonStatus}, feed_event::FeedEvent, utils::{AddedLaterResult, ExpectNone, MaybeRecognizedResult, RemovedLaterResult, StarHelper}};
use crate::utils::{MaybeRecognizedHelper, SometimesMissingHelper, extra_fields_deserialize};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Player {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<String>,

    pub augments: u8,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub bats: MaybeRecognizedResult<Handedness>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub birthday: MaybeRecognizedResult<Day>,
    /// Not present on old, deleted players
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub birthseason: AddedLaterResult<u16>,
    pub durability: f64,
    /// Not present on old, deleted players
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub equipment: AddedLaterResult<PlayerEquipmentMap>,
    /// Not present on old, deleted players
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub feed: AddedLaterResult<Vec<FeedEvent>>,
    pub first_name: String,
    pub last_name: String,
    pub home: String,


    #[serde_as(as = "ExpectNone<_>")]
    greater_boon: Option<serde_json::Value>,
    pub lesser_boon: Option<Boon>,
    pub modifications: Vec<Modification>,

    pub likes: String,
    pub dislikes: String,
    
    pub number: u8,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub position: MaybeRecognizedResult<Position>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub position_type: MaybeRecognizedResult<PositionType>,
    #[serde_as(as = "HashMap<_, HashMap<MaybeRecognizedHelper<_>, _>>")]
    pub season_stats: HashMap<String, HashMap<MaybeRecognizedResult<SeasonStatus>, String>>,
    #[serde_as(as = "HashMap<_, HashMap<MaybeRecognizedHelper<_>, _>>")]
    pub stats: HashMap<String, HashMap<MaybeRecognizedResult<GameStat>, i32>>,

    #[serde(rename = "TeamID")]
    pub team_id: Option<String>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub throws: MaybeRecognizedResult<Handedness>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk: Option<Talk>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

/// A player's equipment field can be described by `HashMap<Result<EquipmentSlot, NotRecognized>, Option<PlayerEquipment>>`
/// 
/// This wrapper is accessed more like `HashMap<Result<EquipmentSlot, NotRecognized>, PlayerEquipment>`, and can be accessed through 
/// an `EquipmentSlot` on its own as well as an `&Result<EquipmentSlot, MaybeRecognized>`.
/// 
/// ```
/// use std::collections::HashMap;
/// use mmolb_parsing::player::{PlayerEquipmentMap, PlayerEquipment};
/// use mmolb_parsing::enums::EquipmentSlot;
/// use mmolb_parsing::NotRecognized;
///  
/// let map = PlayerEquipmentMap::default();
/// map.get(EquipmentSlot::Head);
/// map.get(&Ok(EquipmentSlot::Head));
/// map.get(&Err(NotRecognized(serde_json::Value::String("New Slot".to_string()))));
/// 
/// let a: HashMap<Result<EquipmentSlot, NotRecognized>, PlayerEquipment> = map.clone().into();
/// let b: HashMap<Result<EquipmentSlot, NotRecognized>, Option<PlayerEquipment>> = map.clone().into();
/// ```
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerEquipmentMap {
    #[serde(flatten)]
    #[serde_as(as = "HashMap<MaybeRecognizedHelper<_>, _>")]
    fields: HashMap<MaybeRecognizedResult<EquipmentSlot>, Option<PlayerEquipment>>,
}

impl PlayerEquipmentMap {
    pub fn get<T>(&self, index: T) -> Option<&PlayerEquipment> where Self: _GetHelper<T, Output = PlayerEquipment> {
        self._get(index)
    }
    pub fn get_mut<T>(&mut self, index: T) -> Option<&mut PlayerEquipment> where Self: _GetHelper<T, Output = PlayerEquipment> {
        self._get_mut(index)
    }
}

impl Into<HashMap<MaybeRecognizedResult<EquipmentSlot>, PlayerEquipment>> for PlayerEquipmentMap {
    fn into(self) -> HashMap<MaybeRecognizedResult<EquipmentSlot>, PlayerEquipment> {
        self.fields.into_iter()
            .flat_map(|(slot, equipment)| equipment.and_then(|e| Some((slot, e))))
            .collect()
    }
}

impl Into<HashMap<MaybeRecognizedResult<EquipmentSlot>, Option<PlayerEquipment>>> for PlayerEquipmentMap {
    fn into(self) -> HashMap<MaybeRecognizedResult<EquipmentSlot>, Option<PlayerEquipment>> {
        self.fields
    }
}

impl Into<Vec<PlayerEquipment>> for PlayerEquipmentMap {
    fn into(self) -> Vec<PlayerEquipment> {
        self.fields.into_values().flatten().collect()
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
        self.fields.get(&Ok(index)).map(Option::as_ref).flatten()
    }
    fn _get_mut(&mut self, index: EquipmentSlot) -> Option<&mut Self::Output> {
        self.fields.get_mut(&Ok(index)).map(Option::as_mut).flatten()
    }
}

impl _GetHelper<&MaybeRecognizedResult<EquipmentSlot>> for PlayerEquipmentMap {
    type Output = PlayerEquipment;

    fn _get(&self, index: &MaybeRecognizedResult<EquipmentSlot>) -> Option<&Self::Output> {
        self.fields.get(index).map(Option::as_ref).flatten()
    }
    fn _get_mut(&mut self, index: &MaybeRecognizedResult<EquipmentSlot>) -> Option<&mut Self::Output> {
        self.fields.get_mut(index).map(Option::as_mut).flatten()
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerEquipment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Vec<MaybeRecognizedHelper<_>>>")]
    pub effects: Option<Vec<MaybeRecognizedResult<EquipmentEffect>>>,
    pub emoji: String,
    /// Removed in the current version of the API
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "RemovedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    slot: RemovedLaterResult<MaybeRecognizedResult<EquipmentSlot>>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub name: MaybeRecognizedResult<ItemType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rare_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u8>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "RemovedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<Option<MaybeRecognizedHelper<_>>>")]
    prefix: RemovedLaterResult<Option<MaybeRecognizedResult<ItemPrefix>>>,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "RemovedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<Option<MaybeRecognizedHelper<_>>>")]
    suffix: RemovedLaterResult<Option<MaybeRecognizedResult<ItemSuffix>>>,

    #[serde_as(as = "Vec<MaybeRecognizedHelper<_>>")]
    pub suffixes: Vec<MaybeRecognizedResult<ItemSuffix>>,
    #[serde_as(as = "Vec<MaybeRecognizedHelper<_>>")]
    pub prefixes: Vec<MaybeRecognizedResult<ItemPrefix>>,

    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub rarity: MaybeRecognizedResult<EquipmentRarity>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct EquipmentEffect {
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub attribute: MaybeRecognizedResult<Attribute>,
    #[serde(rename = "Type")]
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub effect_type: MaybeRecognizedResult<EquipmentEffectType>,
    pub value: f64,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Modification {
    pub emoji: String,
    pub name: String,
    pub description: String,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Boon {
    pub emoji: String,
    pub name: String,
    pub description: String,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Talk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batting: Option<TalkCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitching: Option<TalkCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defense: Option<TalkCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baserunning: Option<TalkCategory>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TalkCategory {
    pub quote: String,
    #[serde_as(as = "HashMap<_, StarHelper>")]
    pub stars: HashMap<Attribute, u8>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use crate::{player::Player, utils::{assert_round_trip, no_tracing_errs}};


    #[test]
    fn player_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();

        assert_round_trip::<Player>(Path::new("test_data/player.json"))?;

        drop(no_tracing_errs);
        Ok(())
    }
}