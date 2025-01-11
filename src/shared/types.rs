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
struct MatchDto {
    #[serde(flatten)]
    info: InfoDto,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataDto {
    #[serde(rename = "dataVersion")]
    data_version: String,
    #[serde(rename = "matchId")]
    match_id: String,
    participants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InfoDto {
    participants: Vec<ParticipantDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantDto {
    pub assists: i32,
    #[serde(rename = "championName")]
    pub champion_name: String,
    pub deaths: i32,
    pub kills: i32,
    #[serde(rename = "participantId")]
    pub participant_id: i32,
    pub puuid: String,
    #[serde(rename = "summonerId")]
    pub summoner_id: String,
    #[serde(rename = "summonerName")]
    pub summoner_name: String,
    #[serde(rename = "teamPosition")]
    pub team_position: String,
    pub win: bool,
    #[serde(rename = "riotIdGameName")]
    pub riot_id_game_name: String,
    #[serde(rename = "teamId")]
    pub team_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TeamDto {
    #[serde(rename = "teamId")]
    team_id: i32,
    win: bool,
    // Add other fields as neede
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
