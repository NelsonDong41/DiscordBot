use reqwest::Client;

use crate::shared::{
    requests::request_for_puuid,
    types::{AccountInfoContext, DiscordOutput},
};

pub async fn handle_versus_command(
    champion1: &str,
    champion2: Option<&str>,
    region: Option<&str>,
    api_key: &str,
    client: &Client,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    let puuid_request = request_for_puuid(player_name, tag, region, api_key, client).await;
    let puuid = match puuid_request {
        Ok(puuid) => puuid,
        Err(err) => return Err(err),
    };

    let account_info_context = AccountInfoContext {
        puuid,
        player_name: player_name.to_string(),
        tag: tag.to_string(),
        region: region.to_string(),
    };

    return get_matches_info(game_count, api_key, account_info_context, client).await;
}
