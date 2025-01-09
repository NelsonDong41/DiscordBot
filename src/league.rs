use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize, Debug)]
pub struct AccountDto {
    puuid: String,
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
    participants: Vec<ParticipantDto>,
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
    // Add other fields as neede
}

impl std::error::Error for OutputError {}

pub async fn get_league_info(
    player_name: &str,
    tag: &str,
    region: &str,
    game_count: &str,
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
    game_count: &str,
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
    let region_clone = region.clone();

    let matches_from_puuid_url = format!(
        "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?count={}",
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

            let match_req_urls = match_ids.into_iter().map(|match_id| {
                let match_id_clone = match_id.clone();
                return format!(
                    "https://{}.api.riotgames.com/lol/match/v5/matches/{}",
                    region_clone, match_id_clone
                );
            });

            let match_futures = match_req_urls.map(|url| {
                let client_clone = client.clone();
                let api_key_clone = api_key.to_string();
                async move {
                    let mut retries = 0;
                    let mut delay = Duration::from_millis(500); // Initial delay of 1 second
                    let max_retries = 3;

                    let request = client_clone
                        .get(url.clone())
                        .header("X-Riot-Token", api_key_clone.clone())
                        .build()
                        .unwrap();

                    let mut result = client_clone.execute(request.try_clone().unwrap()).await;

                    result = loop {
                        let status = result.as_ref().unwrap().status();

                        if (result.is_ok() && status.is_success()) || retries >= max_retries {
                            break result;
                        }
                        sleep(delay).await;
                        retries += 1;
                        delay *= 2;

                        result = client_clone.execute(request.try_clone().unwrap()).await;
                    };
                    result
                }
            });

            let match_responses = join_all(match_futures).await;

            let mut matches = Vec::new();
            let mut count = 1;
            for response in match_responses {
                match response {
                    Ok(resp) => {
                        let match_data: serde_json::Value = resp.json().await?;
                        matches.push(get_match_info(match_data, count, puuid.clone()).unwrap());
                        count += 1;
                    }
                    Err(_) => {}
                }
            }

            let matches_len = matches.len();
            let mut total_string = String::new();
            let mut win_count = 0;
            for (match_info, win) in matches {
                total_string.push_str(&match_info);
                total_string.push('\n');
                win_count += if win { 1 } else { 0 };
            }

            Ok(format!(
                "Winrate: {}/{}\n{}",
                win_count,
                matches_len,
                total_string.to_string()
            ))
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
    game_count: i32,
    player_puuid: String,
) -> Result<(String, bool), Box<dyn std::error::Error>> {
    let info: Value = match_resp["info"].clone();
    let participants: Vec<ParticipantDto> =
        match serde_json::from_value(info["participants"].clone()) {
            Ok(participants) => participants,
            Err(_) => Vec::new(),
        };
    let mut output = String::new();
    let participant_iter = participants.iter();

    if participant_iter.len() == 0 {
        return Ok(("Unable to grab participant data".to_string(), false));
    }

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

    Ok((output, me.win))
}
