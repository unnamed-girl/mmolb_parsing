use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde_with::serde_as;

use crate::{enums::{GameStat, PositionType, Slot}, utils::{maybe_recognized_from_str, AddedLaterResult, extra_fields_deserialize, MaybeRecognizedResult}};
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

        // Undrafted player's positions are deeply unreliable
        let filtered_position = (player_id != "#").then(|| maybe_recognized_from_str(&position));

        TeamPlayer { emoji, first_name, last_name, number, player_id, actual_position: position, position: filtered_position, slot, position_type, stats, extra_fields }
    }
}

impl From<TeamPlayer> for RawTeamPlayer {
    fn from(value: TeamPlayer) -> Self {
        let TeamPlayer { emoji, first_name, last_name, number, player_id, actual_position, position: _, slot, position_type, stats, extra_fields } = value;

        RawTeamPlayer { emoji, first_name, last_name, number, player_id, position: actual_position, slot, position_type, stats, extra_fields }
    }
}
