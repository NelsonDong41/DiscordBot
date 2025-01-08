use futures::future::join_all;
use reqwest::{Client, Error};
use serde::Deserialize;
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
            "({}) {}#{}: ({}) {}",
            self.region, self.player_name, self.tag, self.status, self.region
        )
    }
}

impl From<Error> for OutputError {
    fn from(err: Error) -> Self {
        OutputError {
            status: err
                .status()
                .map_or("Unknown".to_string(), |s| s.to_string()),
            message: "Request error".to_string(),
            player_name: "".to_string(), // You might want to pass these values as parameters
            tag: "".to_string(),         // You might want to pass these values as parameters
            region: "".to_string(),      // You might want to pass these values as parameters
        }
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
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
pub struct InfoDto {
    participants: Vec<ParticipantDto>,
}

#[derive(Deserialize, Debug)]
pub struct MatchDto {
    info: InfoDto,
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
    println!("enter1");

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

    println!("enter2");

    match response_status {
        reqwest::StatusCode::OK => {
            let puuid = response.json::<AccountDto>().await?.puuid;

            println!("{}", puuid);

            let account_info_context = AccountInfoContext {
                puuid,
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            };
            println!("enter5");

            return get_matches_info(game_count, api_key, client, account_info_context).await;
        }
        _ => {
            println!("enter6");
            let resp = response.json::<ErrorDto>().await?;
            println!("enter7");

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
    game_count: i64,
    api_key: &str,
    client: &Client,
    account_info_context: AccountInfoContext,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("here1");

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

    println!("here2");

    let request = client
        .get(matches_from_puuid_url)
        .header("X-Riot-Token", api_key)
        .build()
        .unwrap();

    let response = client.execute(request).await?;
    let response_status = response.status();

    println!("here3");

    match response_status {
        reqwest::StatusCode::OK => {
            let match_ids = response.json::<Vec<String>>().await?;
            println!("{}", match_ids.join("\n"));

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
                            let resp_body = resp.text().await.expect("");
                            let match_info: MatchDto = serde_json::from_str(&resp_body).expect("");
                            let match_info_new: Value = serde_json::from_str(&resp_body).expect("");
                            println!("{}", serde_json::to_string_pretty(&match_info_new).unwrap());

                            Ok(match_info)
                        }
                        Err(err) => {
                            println!("request to find matches failed");
                            Err(Box::new(OutputError {
                                status: err.status().unwrap().to_string(),
                                message: "Request to find matches failed".to_string(),
                                player_name: player_name.to_string(),
                                tag: tag.to_string(),
                                region: region.to_string(),
                            }))
                        }
                    }
                }
            });

            let match_info_results: Vec<Result<MatchDto, Box<OutputError>>> =
                join_all(match_info_requests).await;

            let mut total_string = String::new();
            for result in match_info_results {
                match result {
                    Ok(match_info) => {
                        let match_string = get_match_info(Ok(match_info))
                            .unwrap_or_else(|_| "Error getting match info".to_string());
                        total_string.push_str(&match_string);
                        total_string.push('\n'); // Add a newline for better readability
                    }
                    Err(_) => {
                        total_string.push_str("Error getting match info");
                        total_string.push('\n'); // Add a newline for better readability
                    }
                }
            }
            println!("{}", total_string);
            Ok("asdfafdsf".to_string())
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
    match_resp: Result<MatchDto, Box<OutputError>>,
) -> Result<String, Box<dyn std::error::Error>> {
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
