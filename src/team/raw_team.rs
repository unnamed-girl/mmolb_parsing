use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoDiscriminant};
use tracing::error;

use crate::{enums::{GameStat, MaybeRecognized, PositionType, Slot}, team::TeamPlayer, serde_utils::APIHistory};


#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawTeamPlayer {
    pub emoji: String,
    pub first_name: String,
    pub last_name: String,
    pub number: u8,
    #[serde(rename = "PlayerID")]
    pub player_id: String,
    pub position: String,
    pub slot: MaybeRecognized<Slot>,
    #[serde(default)]
    pub position_type: PositionTypeHistory,

    #[serde(default)]
    pub stats: HashMap<MaybeRecognized<GameStat>, i32>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawTeamPlayer> for TeamPlayer {
    fn from(value: RawTeamPlayer) -> Self {
        let RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields } = value;

        let position_type_format = position_type.discriminant();
        let position_type = match position_type {
            PositionTypeHistory::Season0a => MaybeRecognized::<PositionType>::from(position.as_str()),
            PositionTypeHistory::Season0b(position) => position
        };

        // Undrafted player's positions are just their slot
        let position = (player_id != "#").then(|| position.as_str().into());

        if extra_fields.len() > 0 {
            error!("Deserialization found extra fields: {:?}", extra_fields)
        }


        TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields, position_type_format }
    }
}

impl From<TeamPlayer> for RawTeamPlayer {
    fn from(value: TeamPlayer) -> Self {
        let TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields, position_type_format } = value;

        let position = match position {
            Some(position) => position.to_string(),
            None => slot.to_string()
        };

        let position_type = match position_type_format {
            PositionTypeHistoryDiscriminants::Season0a => PositionTypeHistory::Season0a,
            PositionTypeHistoryDiscriminants::Season0b => PositionTypeHistory::Season0b(position_type)
        };

        RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum PositionTypeHistory {
    #[default]
    Season0a,
    Season0b(MaybeRecognized<PositionType>)
}

impl APIHistory for PositionTypeHistory {
    fn is_missing(&self) -> bool {
        matches!(self, Self::Season0a)
    }
}
