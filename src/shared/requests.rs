use super::{
    types::{AccountDto, AccountInfoContext, OutputError},
    util::retry_requests,
};
use reqwest::{Client, Error, Response};

pub async fn send_request(url: &str, api_key: &str, client: &Client) -> Result<Response, Error> {
    let request = client
        .get(url)
        .header("X-Riot-Token", api_key)
        .build()
        .unwrap();
    retry_requests(request, client).await
}

pub async fn request_for_puuid(
    player_name: &str,
    tag: &str,
    region: &str,
    api_key: &str,
    client: &Client,
) -> Result<String, Box<OutputError>> {
    let account_url = format!(
        "https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}",
        region, player_name, tag
    );

    let response = send_request(account_url.as_str(), api_key, client)
        .await
        .unwrap();
    let response_status = response.status();
    match response_status {
        reqwest::StatusCode::OK => {
            let puuid = response.json::<AccountDto>().await.unwrap().puuid;

            return Ok(puuid);
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

pub async fn request_matches_from_puuid(
    game_count: &str,
    api_key: &str,
    account_info_context: AccountInfoContext,
    client: &Client,
) -> Result<Vec<String>, Box<OutputError>> {
    let AccountInfoContext {
        region,
        puuid,
        player_name,
        tag,
    } = account_info_context;
    let matches_from_puuid_url = format!(
        "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?count={}",
        region, puuid, game_count
    );

    let response = send_request(matches_from_puuid_url.as_str(), api_key, client)
        .await
        .unwrap();
    let response_status = response.status();

    match response_status {
        reqwest::StatusCode::OK => {
            let match_ids = response.json::<Vec<String>>().await.unwrap();

            return Ok(match_ids);
        }
        _ => {
            return Err(Box::new(OutputError {
                status: response_status.to_string(),
                message: "Request to find matches failed".to_string(),
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            }));
        }
    }
}
