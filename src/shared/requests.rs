use super::{
    types::{AccountDto, AccountInfoContext},
    util::retry_requests,
};
use reqwest::{Client, Error, Response};

pub async fn send_request(
    url: &str,
    api_key: Option<&str>,
    client: &Client,
) -> Result<Response, Error> {
    let mut request = client.get(url);
    if api_key.is_some() {
        request = request.header("X-Riot-Token", api_key.expect(""));
    }
    let request = request.build().unwrap();
    retry_requests(request, client).await
}

pub async fn request_for_puuid(
    player_name: &str,
    tag: &str,
    region: &str,
    api_key: &str,
    client: &Client,
) -> Result<String, Error> {
    let account_url = format!(
        "https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}",
        region, player_name, tag
    );

    let response = send_request(account_url.as_str(), Some(api_key), client)
        .await
        .unwrap();

    let puuid = response.json::<AccountDto>().await.unwrap().puuid;

    return Ok(puuid);
}

pub async fn request_matches_from_puuid(
    game_count: i64,
    api_key: &str,
    account_info_context: AccountInfoContext,
    client: &Client,
) -> Result<Vec<String>, Error> {
    let AccountInfoContext { region, puuid, .. } = account_info_context;
    let matches_from_puuid_url = format!(
        "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?count={}",
        region, puuid, game_count
    );

    let response = send_request(matches_from_puuid_url.as_str(), Some(api_key), client)
        .await
        .unwrap();
    let match_ids = response.json::<Vec<String>>().await.unwrap();
    return Ok(match_ids);
}
