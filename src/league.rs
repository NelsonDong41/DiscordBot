use reqwest::Client;
use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AccountDto {
    puuid: String,
}

#[derive(Debug)]
pub struct CouldNotPlayer {
    message: String,
    player_name: String,
    tag: String,
    region: String,
}

impl Display for CouldNotPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: Could not find player {}#{} in {}",
            self.message, self.player_name, self.tag, self.region
        )
    }
}

impl std::error::Error for CouldNotPlayer {}

pub async fn get_league_info(
    player_name: &str,
    tag: &str,
    region: &str,
    api_key: &str,
    client: &Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let account_url = format!(
        "https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}",
        region, player_name, tag
    );

    // Make the request we will call
    let request = client
        .get(account_url)
        .header("X-Riot-Token", api_key)
        .build()
        .unwrap();
    // Execute the request and await a JSON result that will be converted to a
    // vector of locations
    let resp = match client.execute(request).await {
        Ok(response) => match response.json::<AccountDto>().await {
            Ok(data) => data,
            Err(_) => {
                return Err(Box::new(CouldNotPlayer {
                    message: format!(
                        "Could not parse json to AccountDto, {:?}",
                        response.text().await.unwrap()
                    ),
                    player_name: player_name.to_string(),
                    tag: tag.to_string(),
                    region: region.to_string(),
                }))
            }
        },
        Err(x) => {
            return Err(Box::new(CouldNotPlayer {
                message: format!("Request failed: {}", x.to_string()),
                player_name: player_name.to_string(),
                tag: tag.to_string(),
                region: region.to_string(),
            }))
        }
    };

    Ok(resp.puuid)
}
