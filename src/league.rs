use futures::future::join_all;
use reqwest::{Client, Request};
use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
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
    message: String,
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
            "({}) {}#{}: ({}) {}",
            self.region, self.player_name, self.tag, self.status, self.region
        )
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AccountInfoContext {
    puuid: String,
    player_name: String,
    tag: String,
    region: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ParticipantDto {
    gold_earned: u32,
    individual_position: String,
    kills: u32,
    deaths: u32,
    assists: u32,
    participant_id: u32,
    summoner_name: String,
    total_damage_dealt: u32,
    win: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct InfoDto {
    participants: Vec<ParticipantDto>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MatchDto {
    info: InfoDto,
}

impl std::error::Error for OutputError {}

pub async fn get_league_info(
    player_name: &str,
    tag: &str,
    region: &str,
    game_count: u32,
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
                puuid: puuid,
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            };
            return get_matches_info(game_count, api_key, client, account_info_context).await;
        }
        _ => {
            let resp = response.json::<ErrorDto>().await?;
            return Err(Box::new(OutputError {
                status: resp.status_code,
                message: resp.message,
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            }));
        }
    }
}

async fn get_matches_info(
    game_count: u32,
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
    let summoner_url = format!(
        "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?count{}",
        region, puuid, game_count
    );

    let request = client
        .get(summoner_url)
        .header("X-Riot-Token", api_key)
        .build()
        .unwrap();

    let response = client.execute(request).await?;
    let response_status = response.status();

    match response_status {
        reqwest::StatusCode::OK => {
            let match_ids = response.json::<Vec<String>>().await?;
            let match_info_requests: Result<MatchDto, OutputError> = match_ids
                .iter()
                .map(|match_id| async {
                    let match_request_url = format!(
                        "https://{}.api.riotgames.com/lol/match/v5/matches/{}",
                        region, match_id
                    );

                    let response = client
                        .get(match_request_url)
                        .header("X-Riot-Token", api_key)
                        .send()
                        .await;

                    match response {
                        Ok(resp) => {
                            let match_info = resp.json::<MatchDto>().await;
                            Ok(match_info)
                        }
                        Err(err) => {
                            let error_json = err.json::<ErrorDto>().await;

                            Err(Box::new(OutputError {
                                status: response_status.to_string(),
                                message: er,
                                player_name: player_name.to_string(),
                                tag: tag.to_string(),
                                region: region.to_string(),
                            }))
                        }
                    }
                })
                .collect();

            return Ok("string");
        }
        _ => {
            let resp = response.json::<ErrorDto>().await?;
            return Err(Box::new(OutputError {
                status: resp.status_code,
                message: resp.message,
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            }));
        }
    }
}

async fn get_match_info(
    client: &Client,
    match_request: Request,
) -> Result<String, Box<dyn std::error::Error>> {
    let match_resp = client
        .execute(match_request)
        .await?
        .json::<MatchDto>()
        .await;

    match match_resp {
        Ok(resp) => {
            let participants = resp.info.participants;
            let mut output = String::new();
            for participant in participants {
                output.push_str(&format!(
                    "Player: {}, Position: {}, Kills: {}, Deaths: {}, Assists: {}, Damage: {}, Win: {}\n",
                    participant.summoner_name,
                    participant.individual_position,
                    participant.kills,
                    participant.deaths,
                    participant.assists,
                    participant.total_damage_dealt,
                    participant.win
                ));
            }
            Ok(output)
        }
        Err(err) => Err(Box::new(err)),
    }
}
