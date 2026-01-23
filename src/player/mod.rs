pub use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

use crate::enums::{AttributeCategory, PitchCategory, PitchType};
use crate::utils::{extra_fields_deserialize, MaybeRecognizedHelper, SometimesMissingHelper};
use crate::{
    enums::{
        Attribute, Day, EquipmentEffectType, EquipmentRarity, EquipmentSlot, GameStat, Handedness,
        ItemName, ItemPrefix, ItemSuffix, Position, PositionType, SeasonStatus, SpecialItemType,
    },
    feed_event::FeedEvent,
    utils::{
        AddedLaterResult, MaybeRecognizedResult, PitchTypeAcronymHelper, RemovedLaterResult,
        StarHelper,
    },
    EmptyArrayOr,
};

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
    pub birthseason: u16,
    pub durability: f64,
    /// Not present on old, deleted players
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub equipment: AddedLaterResult<PlayerEquipmentMap>,

    /// Not present on old, deleted players. No longer present on s4 players
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub feed: AddedLaterResult<Vec<FeedEvent>>,
    pub first_name: String,
    pub last_name: String,
    pub home: String,

    pub greater_boon: BoonCollection,
    pub lesser_boon: BoonCollection,
    pub modifications: Vec<Modification>,

    pub likes: String,
    pub dislikes: String,

    pub number: u8,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub position: MaybeRecognizedResult<Position>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub position_type: MaybeRecognizedResult<PositionType>,

    #[serde_as(as = "EmptyArrayOr<HashMap<_, HashMap<MaybeRecognizedHelper<_>, _>>>")]
    pub season_stats:
        EmptyArrayOr<HashMap<String, HashMap<MaybeRecognizedResult<SeasonStatus>, String>>>,
    #[serde_as(as = "HashMap<_, HashMap<MaybeRecognizedHelper<_>, _>>")]
    pub stats: HashMap<String, HashMap<MaybeRecognizedResult<GameStat>, i32>>,

    #[serde(rename = "TeamID")]
    pub team_id: Option<String>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub throws: MaybeRecognizedResult<Handedness>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk: Option<Talk>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub attribute_stars:
        AddedLaterResult<HashMap<AttributeCategory, HashMap<Attribute, ComplexTalkStars>>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub base_attributes: AddedLaterResult<BaseAttributes>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err",
        rename = "XP"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub xp: AddedLaterResult<u32>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub level: AddedLaterResult<u32>,
    /// E.g. "IV"
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub suffix: AddedLaterResult<Option<String>>,
    /// This is less precise than BaseAttributes.pitch_selection
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub pitch_selection: AddedLaterResult<Vec<f64>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Vec<MaybeRecognizedHelper<PitchTypeAcronymHelper>>>")]
    pub pitch_types: AddedLaterResult<Vec<MaybeRecognizedResult<PitchType>>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(
        as = "SometimesMissingHelper<HashMap<MaybeRecognizedHelper<PitchTypeAcronymHelper>, _>>"
    )]
    pub pitch_type_bonuses: AddedLaterResult<HashMap<MaybeRecognizedResult<PitchType>, f64>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<HashMap<MaybeRecognizedHelper<_>, _>>")]
    pub pitch_category_bonuses:
        AddedLaterResult<HashMap<MaybeRecognizedResult<PitchCategory>, f64>>,

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
    pub inner: HashMap<MaybeRecognizedResult<EquipmentSlot>, Option<PlayerEquipment>>,
}

impl PlayerEquipmentMap {
    pub fn get<T>(&self, index: T) -> Option<&PlayerEquipment>
    where
        Self: _GetHelper<T, Output = PlayerEquipment>,
    {
        self._get(index)
    }
    pub fn get_mut<T>(&mut self, index: T) -> Option<&mut PlayerEquipment>
    where
        Self: _GetHelper<T, Output = PlayerEquipment>,
    {
        self._get_mut(index)
    }
}

impl From<PlayerEquipmentMap> for HashMap<MaybeRecognizedResult<EquipmentSlot>, PlayerEquipment> {
    fn from(val: PlayerEquipmentMap) -> Self {
        val.inner
            .into_iter()
            .flat_map(|(slot, equipment)| equipment.map(|e| (slot, e)))
            .collect()
    }
}

impl From<PlayerEquipmentMap>
    for HashMap<MaybeRecognizedResult<EquipmentSlot>, Option<PlayerEquipment>>
{
    fn from(val: PlayerEquipmentMap) -> Self {
        val.inner
    }
}

impl From<PlayerEquipmentMap> for Vec<PlayerEquipment> {
    fn from(val: PlayerEquipmentMap) -> Self {
        val.inner.into_values().flatten().collect()
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
        self.inner.get(&Ok(index)).and_then(Option::as_ref)
    }
    fn _get_mut(&mut self, index: EquipmentSlot) -> Option<&mut Self::Output> {
        self.inner.get_mut(&Ok(index)).and_then(Option::as_mut)
    }
}

impl _GetHelper<&MaybeRecognizedResult<EquipmentSlot>> for PlayerEquipmentMap {
    type Output = PlayerEquipment;

    fn _get(&self, index: &MaybeRecognizedResult<EquipmentSlot>) -> Option<&Self::Output> {
        self.inner.get(index).and_then(Option::as_ref)
    }
    fn _get_mut(
        &mut self,
        index: &MaybeRecognizedResult<EquipmentSlot>,
    ) -> Option<&mut Self::Output> {
        self.inner.get_mut(index).and_then(Option::as_mut)
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
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "RemovedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    pub slot: RemovedLaterResult<MaybeRecognizedResult<EquipmentSlot>>,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub name: MaybeRecognizedResult<ItemName>,

    #[serde(rename = "Type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_type: Option<SpecialItemType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rare_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u8>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub durability: AddedLaterResult<u32>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub prefix_position_type: AddedLaterResult<PositionType>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub specialized: AddedLaterResult<bool>,

    /// Only exists on deleted player's equipment. Was replaced with the "prefixes" field once multi-prefix items
    /// were added.
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "RemovedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Option<MaybeRecognizedHelper<_>>>")]
    pub prefix: RemovedLaterResult<Option<MaybeRecognizedResult<ItemPrefix>>>,

    /// Only exists on deleted player's equipment. Was replaced with the "suffixes" field once multi-suffix items
    /// were added.
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "RemovedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Option<MaybeRecognizedHelper<_>>>")]
    pub suffix: RemovedLaterResult<Option<MaybeRecognizedResult<ItemSuffix>>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Vec<MaybeRecognizedHelper<_>>>")]
    pub suffixes: AddedLaterResult<Vec<MaybeRecognizedResult<ItemSuffix>>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Vec<MaybeRecognizedHelper<_>>>")]
    pub prefixes: AddedLaterResult<Vec<MaybeRecognizedResult<ItemPrefix>>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    pub rarity: AddedLaterResult<MaybeRecognizedResult<EquipmentRarity>>,

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

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub tier: AddedLaterResult<u32>,

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

    // Reflection players' talk entries have a null day
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Option<MaybeRecognizedHelper<_>>>")]
    pub day: AddedLaterResult<Option<MaybeRecognizedResult<Day>>>,

    // Reflection players' talk entries have a null season
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub season: AddedLaterResult<Option<u8>>,
    pub stars: HashMap<Attribute, TalkStars>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<HashMap<_, _>>")]
    pub attributes: AddedLaterResult<HashMap<Attribute, f64>>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TalkStars {
    Complex(ComplexTalkStars),
    Intermediate {
        display: String,
        regular: u8,
        shiny: u8,
        stars: u8,
        total: f64,
    },
    Simple(#[serde_as(as = "StarHelper")] u8),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComplexTalkStars {
    pub attribute: Attribute,
    pub display: String,
    pub regular: u8,
    pub shiny: u8,
    pub stars: u8,
    pub total: f64,
    pub base_display: String,
    pub base_regular: u8,
    pub base_shiny: u8,
    pub base_stars: u8,
    pub base_total: f64,
}

/// In season 10, Lesser and Greater boons moved from Option<Modification> to Option<Vec<Modification>>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(untagged)]
pub enum BoonCollection {
    /// Null
    #[default]
    None,
    Single(Modification),
    Many(Vec<Modification>),
}

impl BoonCollection {
    pub fn iter(&'_ self) -> BoonCollectionRefIterator<'_> {
        match self {
            BoonCollection::None => BoonCollectionRefIterator::None(std::iter::empty()),
            BoonCollection::Single(modification) => {
                BoonCollectionRefIterator::Single(std::iter::once(modification))
            }
            BoonCollection::Many(modifications) => {
                BoonCollectionRefIterator::Many(modifications.iter())
            }
        }
    }

    pub fn iter_mut(&'_ mut self) -> BoonCollectionMutIterator<'_> {
        match self {
            BoonCollection::None => BoonCollectionMutIterator::None(std::iter::empty()),
            BoonCollection::Single(modification) => {
                BoonCollectionMutIterator::Single(std::iter::once(modification))
            }
            BoonCollection::Many(modifications) => {
                BoonCollectionMutIterator::Many(modifications.iter_mut())
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            BoonCollection::None => 0,
            BoonCollection::Single(_modification) => 1,
            BoonCollection::Many(modifications) => modifications.len(),
        }
    }
}

pub enum BoonCollectionIterator {
    None(std::iter::Empty<Modification>),
    Single(std::iter::Once<Modification>),
    Many(std::vec::IntoIter<Modification>),
}

impl Iterator for BoonCollectionIterator {
    type Item = Modification;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BoonCollectionIterator::None(empty) => empty.next(),
            BoonCollectionIterator::Single(once) => once.next(),
            BoonCollectionIterator::Many(many) => many.next(),
        }
    }
}

pub enum BoonCollectionRefIterator<'a> {
    None(std::iter::Empty<&'a Modification>),
    Single(std::iter::Once<&'a Modification>),
    Many(core::slice::Iter<'a, Modification>),
}

impl<'a> Iterator for BoonCollectionRefIterator<'a> {
    type Item = &'a Modification;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BoonCollectionRefIterator::None(empty) => empty.next(),
            BoonCollectionRefIterator::Single(once) => once.next(),
            BoonCollectionRefIterator::Many(many) => many.next(),
        }
    }
}

pub enum BoonCollectionMutIterator<'a> {
    None(std::iter::Empty<&'a mut Modification>),
    Single(std::iter::Once<&'a mut Modification>),
    Many(core::slice::IterMut<'a, Modification>),
}

impl<'a> Iterator for BoonCollectionMutIterator<'a> {
    type Item = &'a mut Modification;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BoonCollectionMutIterator::None(empty) => empty.next(),
            BoonCollectionMutIterator::Single(once) => once.next(),
            BoonCollectionMutIterator::Many(many) => many.next(),
        }
    }
}

impl IntoIterator for BoonCollection {
    type Item = Modification;
    type IntoIter = BoonCollectionIterator;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            BoonCollection::None => BoonCollectionIterator::None(std::iter::empty()),
            BoonCollection::Single(modification) => {
                BoonCollectionIterator::Single(std::iter::once(modification))
            }
            BoonCollection::Many(modifications) => {
                BoonCollectionIterator::Many(modifications.into_iter())
            }
        }
    }
}

impl<'a> IntoIterator for &'a BoonCollection {
    type Item = &'a Modification;
    type IntoIter = BoonCollectionRefIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut BoonCollection {
    type Item = &'a mut Modification;
    type IntoIter = BoonCollectionMutIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct BaseAttributes {
    #[serde(flatten)]
    pub attributes: HashMap<Attribute, f64>,
    #[serde_as(as = "Vec<MaybeRecognizedHelper<PitchTypeAcronymHelper>>")]
    pub pitch_types: Vec<MaybeRecognizedResult<PitchType>>,
    pub pitch_selection: Vec<f64>,
}

#[cfg(test)]
mod test {
    use crate::{
        player::Player,
        utils::{assert_round_trip, no_tracing_errs},
    };
    use std::path::Path;

    #[test]
    fn player_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();

        assert_round_trip::<Player>(Path::new("test_data/player.json"))?;

        drop(no_tracing_errs);
        Ok(())
    }
}
