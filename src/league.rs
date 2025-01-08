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
    metadata: MetadataDto,
    info: InfoDto,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataDto {
    data_version: String,
    match_id: String,
    participants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InfoDto {
    game_creation: i64,
    game_duration: i64,
    game_end_timestamp: i64,
    game_id: i64,
    game_mode: String,
    game_name: String,
    game_start_timestamp: i64,
    game_type: String,
    game_version: String,
    map_id: i32,
    participants: Vec<ParticipantDto>,
    platform_id: String,
    queue_id: i32,
    teams: Vec<TeamDto>,
    tournament_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParticipantDto {
    assists: i32,
    champion_id: i32,
    deaths: i32,
    kills: i32,
    participant_id: i32,
    puuid: String,
    summoner_id: String,
    summoner_name: String,
    // Add other fields as needed
}

#[derive(Debug, Serialize, Deserialize)]
struct TeamDto {
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

            let match_info_requests = match_ids.iter().map(|match_id| {
                let client = client.clone();
                let region = region.clone();
                let player_name = player_name.clone();
                let tag = tag.clone();
                async move {
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
                            let match_info_result: Result<MatchDto, Error> = resp.json().await;

                            match match_info_result {
                                Ok(match_info) => Ok(match_info),
                                Err(_) => {
                                    println!("tag");

                                    Err(Box::new(OutputError {
                                        status: response_status.to_string(),
                                        message: "Error parsing json response to MatchDto"
                                            .to_string(),
                                        player_name: player_name.to_string(),
                                        tag: tag.to_string(),
                                        region: region.to_string(),
                                    }))
                                }
                            }
                        }
                        Err(err) => Err(Box::new(OutputError {
                            status: err.status().unwrap().to_string(),
                            message: "Request to find matches failed".to_string(),
                            player_name: player_name.to_string(),
                            tag: tag.to_string(),
                            region: region.to_string(),
                        })),
                    }
                }
            });

            let match_info_results: Vec<Result<MatchDto, Box<OutputError>>> =
                join_all(match_info_requests).await;

            let mut total_string = String::new();
            let mut count = 1;
            println!("before1");

            for result in match_info_results {
                match result {
                    Ok(match_info) => {
                        println!("before");
                        let match_string = get_match_info(match_info, count, puuid.clone())
                            .unwrap_or_else(|err| err.to_string());
                        total_string.push_str(&match_string);
                        total_string.push('\n'); // Add a newline for better readability
                    }
                    Err(err) => {
                        total_string.push_str(&err.message);
                        total_string.push('\n'); // Add a newline for better readability
                    }
                }
                count += 1;
            }
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
    match_resp: MatchDto,
    game_count: i64,
    player_puuid: String,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("hello");

    let info = match_resp.info;
    let InfoDto {
        // participants,
        game_duration,
        ..
    } = info;
    let mut output = String::new();
    // let participant_iter = participants.iter();

    // println!("hello0");

    // let me = participant_iter
    //     .clone()
    //     .find(|p| p.puuid == player_puuid)
    //     .unwrap();

    // println!("hello1");

    // let opponent = participant_iter
    //     .clone()
    //     .find(|p| p.individual_position != me.individual_position)
    //     .unwrap();

    // println!("hello2");

    // let win = if me.win { "won" } else { "lost" };
    // output.push_str(&format!(
    //     "Game {} {}: {} (you) vs {} ({})\n",
    //     game_count, win, me.summoner_name, opponent.summoner_name, game_duration
    // ));

    Ok(game_duration.to_string())
}
