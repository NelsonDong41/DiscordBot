use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serenity::all::{Colour, CreateEmbedFooter};

#[derive(Deserialize, Debug)]
pub struct AccountDto {
    pub puuid: String,
}

#[derive(Debug)]
pub struct OutputError {
    pub status: String,
    pub message: String,
    pub player_name: String,
    pub tag: String,
    pub region: String,
}

impl Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} \n ({}) {}#{}: ({})",
            self.message, self.status, self.player_name, self.tag, self.region
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfoContext {
    pub puuid: String,
    pub player_name: String,
    pub tag: String,
    pub region: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchDto {
    pub info: InfoDto,
    pub metadata: MetadataDto,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataDto {
    data_version: String,
    match_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoDto {
    pub participants: Vec<ParticipantDto>,
    pub game_mode: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantDto {
    pub assists: i32,
    pub champion_name: String,
    pub deaths: i32,
    pub kills: i32,
    pub participant_id: i32,
    pub puuid: String,
    pub summoner_id: String,
    pub summoner_name: String,
    pub team_position: String,
    pub win: bool,
    pub riot_id_game_name: String,
    pub team_id: u32,
}

impl std::error::Error for OutputError {}

pub struct DiscordOutput {
    pub color: Colour,
    pub description: String,
    pub fields: Vec<(String, String, bool)>,
    pub footer: CreateEmbedFooter,
    pub title: String,
    pub content: String,
}

impl DiscordOutput {
    pub fn new(
        color: Colour,
        description: String,
        fields: Vec<(String, String, bool)>,
        footer: CreateEmbedFooter,
        title: String,
        content: String,
    ) -> Self {
        DiscordOutput {
            color,
            description,
            fields,
            footer,
            title,
            content,
        }
    }
}
