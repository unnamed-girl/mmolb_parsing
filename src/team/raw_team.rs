use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use super::team::TeamPlayer;
use crate::player::{BoonCollection, FoodBuff, Modification, PendingLevelUp};
use crate::utils::{MaybeRecognizedHelper, SometimesMissingHelper};
use crate::{
    enums::{BenchRole, FullSlot, FullSlotLabel, GameStat, PositionType, SlotType},
    utils::{
        extra_fields_deserialize, maybe_recognized_from_str, AddedLaterResult,
        MaybeRecognizedResult,
    },
};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawTeamPlayer {
    pub emoji: String,
    pub first_name: String,
    pub last_name: String,
    /// E.g. "IV"
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub suffix: AddedLaterResult<Option<String>>,
    pub number: u8,
    #[serde(rename = "PlayerID")]
    pub player_id: String,
    pub position: String,
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub slot_type: AddedLaterResult<MaybeRecognizedResult<SlotType>>,
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub slot: AddedLaterResult<MaybeRecognizedResult<FullSlot>>,
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub slot_label: AddedLaterResult<MaybeRecognizedResult<FullSlotLabel>>,
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub position_type: AddedLaterResult<MaybeRecognizedResult<PositionType>>,

    #[serde_as(as = "SometimesMissingHelper<HashMap<MaybeRecognizedHelper<_>, _>>")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    pub stats: AddedLaterResult<HashMap<MaybeRecognizedResult<GameStat>, i32>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub bench_index: AddedLaterResult<Option<u32>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub bench_role: AddedLaterResult<Option<BenchRole>>,

    // Added in s11
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub food_buffs: AddedLaterResult<Vec<FoodBuff>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub greater_boon: AddedLaterResult<BoonCollection>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub lesser_boon: AddedLaterResult<BoonCollection>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub modifications: AddedLaterResult<Vec<Modification>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub level: AddedLaterResult<u32>,

    // Added in s11
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub pending_level_ups: AddedLaterResult<Vec<PendingLevelUp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_recomp: Option<bool>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawTeamPlayer> for TeamPlayer {
    fn from(value: RawTeamPlayer) -> Self {
        let RawTeamPlayer {
            emoji,
            first_name,
            last_name,
            suffix,
            number,
            player_id,
            position,
            slot_type,
            slot,
            slot_label,
            position_type,
            stats,
            extra_fields,
            bench_index,
            bench_role,
            food_buffs,
            greater_boon,
            lesser_boon,
            modifications,
            level,
            pending_level_ups,
            free_recomp,
        } = value;

        // Undrafted player's positions are deeply unreliable
        let filtered_position = (player_id != "#").then(|| maybe_recognized_from_str(&position));

        TeamPlayer {
            emoji,
            first_name,
            last_name,
            suffix,
            number,
            player_id,
            actual_position: position,
            position: filtered_position,
            slot_type,
            slot,
            slot_label,
            position_type,
            stats,
            bench_index,
            bench_role,
            extra_fields,
            food_buffs,
            greater_boon,
            lesser_boon,
            modifications,
            level,
            pending_level_ups,
            free_recomp,
        }
    }
}

impl From<TeamPlayer> for RawTeamPlayer {
    fn from(value: TeamPlayer) -> Self {
        let TeamPlayer {
            emoji,
            first_name,
            last_name,
            suffix,
            number,
            player_id,
            actual_position,
            position: _,
            slot_type,
            slot,
            slot_label,
            position_type,
            stats,
            extra_fields,
            bench_index,
            bench_role,
            food_buffs,
            greater_boon,
            lesser_boon,
            modifications,
            level,
            pending_level_ups,
            free_recomp,
        } = value;

        RawTeamPlayer {
            emoji,
            first_name,
            last_name,
            suffix,
            number,
            player_id,
            position: actual_position,
            slot_type,
            slot,
            slot_label,
            position_type,
            stats,
            bench_index,
            bench_role,
            extra_fields,
            food_buffs,
            greater_boon,
            lesser_boon,
            modifications,
            level,
            pending_level_ups,
            free_recomp,
        }
    }
}
