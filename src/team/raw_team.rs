use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde_with::serde_as;

use crate::{enums::{GameStat, PositionType, Slot}, utils::{maybe_recognized_from_str, maybe_recognized_to_string, AddedLaterResult, extra_fields_deserialize, MaybeRecognizedResult}, AddedLater};
use crate::utils::{MaybeRecognizedHelper, SometimesMissingHelper};
use super::team::TeamPlayer;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawTeamPlayer {
    pub emoji: String,
    pub first_name: String,
    pub last_name: String,
    pub number: u8,
    #[serde(rename = "PlayerID")]
    pub player_id: String,
    pub position: String,
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    pub slot: AddedLaterResult<MaybeRecognizedResult<Slot>>,
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    pub position_type: AddedLaterResult<MaybeRecognizedResult<PositionType>>,

    #[serde_as(as = "SometimesMissingHelper<HashMap<MaybeRecognizedHelper<_>, _>>")]
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    pub stats: AddedLaterResult<HashMap<MaybeRecognizedResult<GameStat>, i32>>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawTeamPlayer> for TeamPlayer {
    fn from(value: RawTeamPlayer) -> Self {
        let RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields } = value;

        let position_type_overidden = position_type.is_err();
        let position_type = position_type.unwrap_or_else(|_| maybe_recognized_from_str(&position));

        // Undrafted player's positions are just their slot
        let position = (player_id != "#").then(|| maybe_recognized_from_str(&position));

        TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, position_type_overridden: position_type_overidden, extra_fields }
    }
}

impl From<TeamPlayer> for RawTeamPlayer {
    fn from(value: TeamPlayer) -> Self {
        let TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, position_type_overridden: position_type_overidden, extra_fields } = value;

        let position = match (position, position_type_overidden, &slot) {
            (Some(position), _, _) => maybe_recognized_to_string(&position),
            (None, true, _) => maybe_recognized_to_string(&position_type),
            (None, false, Ok(slot)) => maybe_recognized_to_string(&slot),
            (None, false, Err(AddedLater)) => panic!("TODO woofy what should I do here?"),
        };

        let position_type = (!position_type_overidden).then_some(position_type).ok_or(AddedLater);

        RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields }
    }
}
