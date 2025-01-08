use futures::future::join_all;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;

#[derive(Deserialize, Debug)]
pub struct AccountDto {
    puuid: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SummonerDto {
    account_id: String,
    profile_icon_id: u32,
    revision_date: u64,
    id: String,
    puuid: String,
    summoner_level: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ErrorDto {
    status_code: String,
}

#[derive(Debug)]
pub struct OutputError {
    status: String,
    message: String,
    player_name: String,
    tag: String,
    region: String,
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfoContext {
    puuid: String,
    player_name: String,
    tag: String,
    region: String,
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
    #[serde(rename = "gameCreation")]
    game_creation: i64,
    #[serde(rename = "gameDuration")]
    game_duration: i64,
    #[serde(rename = "gameEndTimestamp")]
    game_end_timestamp: i64,
    #[serde(rename = "gameId")]
    game_id: i64,
    #[serde(rename = "gameMode")]
    game_mode: String,
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "gameStartTimestamp")]
    game_start_timestamp: i64,
    #[serde(rename = "gameType")]
    game_type: String,
    #[serde(rename = "gameVersion")]
    game_version: String,
    #[serde(rename = "mapId")]
    map_id: i32,
    participants: Vec<ParticipantDto>,
    #[serde(rename = "platformId")]
    platform_id: String,
    #[serde(rename = "queueId")]
    queue_id: i32,
    teams: Vec<TeamDto>,
    #[serde(rename = "tournamentCode")]
    tournament_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParticipantDto {
    assists: i32,
    #[serde(rename = "championName")]
    champion_name: String,
    deaths: i32,
    kills: i32,
    #[serde(rename = "participantId")]
    participant_id: i32,
    puuid: String,
    #[serde(rename = "summonerId")]
    summoner_id: String,
    #[serde(rename = "summonerName")]
    summoner_name: String,
    #[serde(rename = "teamPosition")]
    team_position: String,
    win: bool,
    #[serde(rename = "riotIdGameName")]
    riot_id_game_name: String,
    #[serde(rename = "teamId")]
    team_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TeamDto {
    #[serde(rename = "teamId")]
    team_id: i32,
    win: bool,
    // Add other fields as needed
}

impl std::error::Error for OutputError {}

pub async fn get_league_info(
    player_name: &str,
    tag: &str,
    region: &str,
    game_count: i64,
    api_key: &str,
    client: &Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let account_url = format!(
        "https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}",
        region, player_name, tag
    );

    // // Make the request we will call
    let request = client
        .get(account_url)
        .header("X-Riot-Token", api_key)
        .build()
        .unwrap();
    // // Execute the request and await a JSON result that will be converted to a
    // // vector of locations
    let response = client.execute(request).await?;
    let response_status = response.status();

    match response_status {
        reqwest::StatusCode::OK => {
            let puuid = response.json::<AccountDto>().await?.puuid;

            let account_info_context = AccountInfoContext {
                puuid,
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            };

            return get_matches_info(game_count, api_key, client, account_info_context).await;
        }
        _ => {
            return Err(Box::new(OutputError {
                status: response_status.to_string(),
                message: "Request to find account failed".to_string(),
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            }));
        }
    }
}

async fn get_matches_info(
    game_count: i64,
    api_key: &str,
    client: &Client,
    account_info_context: AccountInfoContext,
) -> Result<String, Box<dyn std::error::Error>> {
    let AccountInfoContext {
        region,
        puuid,
        player_name,
        tag,
    } = account_info_context;
    let matches_from_puuid_url = format!(
        "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?count{}",
        region, puuid, game_count
    );

    let request = client
        .get(matches_from_puuid_url)
        .header("X-Riot-Token", api_key)
        .build()
        .unwrap();

    let response = client.execute(request).await?;
    let response_status = response.status();

    match response_status {
        reqwest::StatusCode::OK => {
            let match_ids = response.json::<Vec<String>>().await?;

            let match_futures = match_ids.into_iter().map(|match_id| {
                let match_url = format!(
                    "https://{}.api.riotgames.com/lol/match/v5/matches/{}",
                    region, match_id
                );
                client
                    .get(&match_url)
                    .header("X-Riot-Token", api_key)
                    .send()
            });

            let match_responses = join_all(match_futures).await;

            let mut matches = Vec::new();
            let mut count = 1;
            for response in match_responses {
                if let Ok(resp) = response {
                    let match_data: serde_json::Value = resp.json().await?;
                    matches.push(
                        get_match_info(match_data, count, puuid.clone())
                            .unwrap_or_else(|err| err.to_string()),
                    );
                    count += 1
                }
            }

            let mut total_string = String::new();
            for match_info in matches {
                total_string.push_str(&match_info);
                total_string.push('\n');
            }

            println!("hello");
            Ok(total_string.to_string())
        }
        _ => {
            let resp = response.json::<ErrorDto>().await?;
            return Err(Box::new(OutputError {
                status: resp.status_code,
                message: "Unable to find matches from puuid".to_string(),
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            }));
        }
    }
}

fn get_match_info(
    match_resp: Value,
    game_count: i64,
    player_puuid: String,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("hello");

    let info: InfoDto = serde_json::from_value(match_resp["info"].clone())?;
    //
    let InfoDto { participants, .. } = info;
    let mut output = String::new();
    let participant_iter = participants.iter();

    let me = participant_iter
        .clone()
        .find(|p| p.puuid == player_puuid)
        .unwrap();

    let opponent = participant_iter
        .clone()
        .find(|p| p.team_id != me.team_id && p.team_position == me.team_position)
        .unwrap();

    let win = if me.win { "won" } else { "lost" };
    output.push_str(&format!(
        "Game {} {}: {} vs {} ({})",
        game_count, win, me.champion_name, opponent.champion_name, opponent.riot_id_game_name
    ));

    Ok(output)
}
