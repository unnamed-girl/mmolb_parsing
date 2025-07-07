use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::{enums::{GameStat, MaybeRecognized, PositionType, Slot}, utils::ExtraFields, AddedLater};
use super::team::TeamPlayer;


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
    pub slot: MaybeRecognized<Slot>,
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub position_type: AddedLater<MaybeRecognized<PositionType>>,

    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub stats: AddedLater<HashMap<MaybeRecognized<GameStat>, i32>>,

    #[serde(flatten)]
    pub extra_fields: ExtraFields,
}

impl From<RawTeamPlayer> for TeamPlayer {
    fn from(value: RawTeamPlayer) -> Self {
        let RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields } = value;

        let position_type_overidden = position_type.is_none();
        let position_type = position_type.into_inner().unwrap_or_else(|| MaybeRecognized::<PositionType>::from(position.as_str()));

        // Undrafted player's positions are just their slot
        let position = (player_id != "#").then(|| position.as_str().into());

        TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, position_type_overidden, extra_fields }
    }
}

impl From<TeamPlayer> for RawTeamPlayer {
    fn from(value: TeamPlayer) -> Self {
        let TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, position_type_overidden, extra_fields } = value;

        let position = match (position, position_type_overidden) {
            (Some(position), _) => position.to_string(),
            (None, true) => position_type.to_string(),
            (None, false) => slot.to_string()
        };

        let position_type = AddedLater((!position_type_overidden).then_some(position_type));

        RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields }
    }
}
