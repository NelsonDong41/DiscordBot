use futures::future::join_all;
use reqwest::Client;
use serenity::all::{Color, CreateEmbedFooter};

use crate::shared::{
    requests::{request_for_puuid, request_matches_from_puuid, send_request},
    types::{AccountInfoContext, DiscordOutput, MatchDto},
};

pub async fn handle_matches_command(
    player_name: &str,
    tag: &str,
    region: &str,
    game_count: i64,
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

async fn get_matches_info(
    game_count: i64,
    api_key: &str,
    account_info_context: AccountInfoContext,
    client: &Client,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    let AccountInfoContext {
        region,
        puuid,
        player_name,
        ..
    } = account_info_context.clone();
    let match_ids =
        request_matches_from_puuid(game_count, api_key, account_info_context, client).await?;

    let match_req_urls: Vec<String> = match_ids
        .into_iter()
        .map(|match_id| {
            return format!(
                "https://{}.api.riotgames.com/lol/match/v5/matches/{}",
                region, match_id
            );
        })
        .collect();

    let match_futures = match_req_urls.clone().into_iter().map(|url| {
        let client_clone = client.clone();
        let api_key_clone = api_key.to_string();
        async move { send_request(url.as_str(), api_key_clone.as_str(), &client_clone).await }
    });

    let match_responses = join_all(match_futures).await;

    let mut matches = Vec::new();
    let mut count = 1;
    for response in match_responses {
        match response {
            Ok(resp) => {
                let match_data = resp.json::<MatchDto>().await?;
                let match_info = get_match_info(match_data, count, puuid.clone()).unwrap();
                matches.push(match_info);
                count += 1;
            }
            Err(_) => {}
        }
    }

    let matches_len: usize = matches.len();
    let match_infos_iter = matches.into_iter();
    let match_infos = match_infos_iter.clone().map(|(info, _)| info);
    let win_count = match_infos_iter
        .clone()
        .fold(0, |acc, (_, win)| acc + win as i32);

    let discord_output = DiscordOutput::new(
        Color::DARK_GREEN,
        format!(
            "Winrate: {}% ({}/{})",
            (win_count as f32 / matches_len as f32) * 100.0,
            win_count,
            matches_len,
        ),
        match_infos.collect(),
        CreateEmbedFooter::new(""),
        format!("{}'s Matches", player_name),
        "".to_string(),
    );

    Ok(discord_output)
}

fn get_match_info(
    match_resp: MatchDto,
    game_count: i32,
    player_puuid: String,
) -> Result<((String, String, bool), bool), Box<dyn std::error::Error>> {
    let info = match_resp.info;
    let participants = info.participants;
    let participant_iter = participants.iter();

    if participant_iter.len() == 0 {
        return Err("No participants found".into());
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
    let me_kda = format!("{}/{}/{}", me.kills, me.deaths, me.assists);
    let opponent_kda = format!(
        "{}/{}/{}",
        opponent.kills, opponent.deaths, opponent.assists
    );
    let output = (
        format!(
            "{}: {} ({})",
            game_count,
            me.team_position.to_uppercase(),
            win.to_uppercase()
        ),
        format!(
            "```({})\n{}\nvs.\n({})\n{}\n({})```",
            me_kda,
            me.champion_name,
            opponent_kda,
            opponent.champion_name,
            opponent.riot_id_game_name
        ),
        true,
    );

    Ok((output, me.win))
}
