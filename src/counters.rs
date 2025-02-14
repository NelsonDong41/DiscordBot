use std::sync::Arc;

use headless_chrome::Tab;
use scraper::{Html, Selector};
use serenity::all::Colour;
use tracing::{info, instrument};

use crate::shared::types::DiscordOutput;

const NUM_CHAMP_COUNTERS: usize = 10;

#[instrument(skip(tab), fields(champion = champion, lane = lane))]
pub async fn handle_counters_command(
    champion: &str,
    lane: Option<&str>,
    tab: &Arc<Tab>,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    info!("handle_counters_command called");
    let document = get_u_gg_document_body(champion, lane, &tab).await.unwrap();

    let lane = get_lane(&document, lane);

    let best_picks_selector = Selector::parse(".w-full.grid div:first-child a").unwrap();
    let worst_picks_selector = Selector::parse(".w-full.grid div:nth-child(2) a").unwrap();
    let lane_picks_selector = Selector::parse(".w-full.grid div:nth-child(3) a").unwrap();

    let name_selector = Selector::parse(".text-white.font-bold.truncate").unwrap();
    let winrate_selector = Selector::parse(".font-bold.whitespace-nowrap.text-right").unwrap();

    let best_picks = &document
        .select(&best_picks_selector)
        .take(NUM_CHAMP_COUNTERS)
        .map(|anchor| {
            let name = anchor
                .select(&name_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("")
                .trim();
            let winrate = anchor
                .select(&winrate_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("");
            format!("{:<10} - {}\n", name, winrate)
        })
        .fold(String::new(), |mut acc, pick_info| {
            acc.push_str(&pick_info);
            acc
        });

    let worst_picks = &document
        .select(&worst_picks_selector)
        .take(NUM_CHAMP_COUNTERS)
        .map(|anchor| {
            let name = anchor
                .select(&name_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("")
                .trim();
            let winrate = anchor
                .select(&winrate_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("");
            format!("{:<10} - {}\n", name, winrate)
        })
        .fold(String::new(), |mut acc, pick_info| {
            acc.push_str(&pick_info);
            acc
        });

    let lane_picks = &document
        .select(&lane_picks_selector)
        .take(NUM_CHAMP_COUNTERS)
        .map(|anchor| {
            let name = anchor
                .select(&name_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("")
                .trim();
            let winrate = anchor
                .select(&winrate_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("");
            format!("{:<10} - {}\n", name, winrate)
        })
        .fold(String::new(), |mut acc, pick_info| {
            acc.push_str(&pick_info);
            acc
        });

    let fields = vec![
        (
            "🟢 - Best Picks".to_string(),
            format!("```{}```", best_picks),
            false,
        ),
        (
            "🔴 - Worst Picks".to_string(),
            format!("```{}```", worst_picks),
            false,
        ),
        (
            "🟡 - Lane Picks".to_string(),
            format!("```{}```", lane_picks),
            false,
        ),
    ];

    Ok(DiscordOutput {
        title: format!("Counter picks for {} ({})", champion, lane),
        description: "".to_string(),
        color: Colour::DARK_GREEN,
        fields,
        footer: "".to_string(),
        content: "".to_string(),
    })
}

#[instrument(skip(tab), fields(champion = champion, lane = lane))]
async fn get_u_gg_document_body(
    champion: &str,
    lane: Option<&str>,
    tab: &Arc<Tab>,
) -> Result<Html, Box<dyn std::error::Error>> {
    info!("get_u_gg_document_body called");

    let mut u_gg_url = String::with_capacity(128);

    u_gg_url.push_str("https://u.gg/lol/champions/");
    u_gg_url.push_str(champion);
    u_gg_url.push_str("/counter");

    if let Some(x) = lane {
        u_gg_url.push_str("/");
        u_gg_url.push_str("?role=");
        u_gg_url.push_str(x);
    }

    tab.navigate_to(&u_gg_url)?;
    tab.wait_until_navigated()?;

    let champion_information = tab.wait_for_element("div#content")?;

    Ok(Html::parse_document(&champion_information.get_content()?))
}

fn get_lane(document: &Html, lane: Option<&str>) -> String {
    info!("get_lane called");

    let result = match lane {
        Some(x) => x.to_string().to_uppercase(),
        None => {
            let lane_selector = Selector::parse(
                ".media-query_MOBILE_SMALL__DESKTOP_SMALL .filter-select .role-value div",
            )
            .unwrap();
            document
                .select(&lane_selector)
                .next()
                .expect("")
                .text()
                .next()
                .expect("")
                .trim()
                .to_string()
        }
    };

    info!("get_lane result: result = {:?}", result);

    result
}
