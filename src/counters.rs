use std::sync::Arc;

use headless_chrome::Tab;
use scraper::{Html, Selector};
use tracing::instrument;

use crate::shared::types::DiscordOutput;

#[instrument(skip(tab), fields(champion = champion, lane = lane))]
pub async fn handle_build_command(
    champion: &str,
    lane: Option<&str>,
    tab: &Arc<Tab>,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    let document = get_u_gg_document_body(champion, lane, &tab).await;

    let best_picks_selector = Selector::parse(".w-full grid .gap-[12px] .grid-cols-[repeat(3,1fr)] .max-[900px]:grid-cols-[repeat(2,1fr)] .max-[600px]:grid-cols-1 div:first-child a").unwrap();
    let worst_picks_selector = Selector::parse(".w-full grid .gap-[12px] .grid-cols-[repeat(3,1fr)] .max-[900px]:grid-cols-[repeat(2,1fr)] .max-[600px]:grid-cols-1 div:nth-child(2) a").unwrap();
    let lane_picks_selector = Selector::parse(".w-full grid .gap-[12px] .grid-cols-[repeat(3,1fr)] .max-[900px]:grid-cols-[repeat(2,1fr)] .max-[600px]:grid-cols-1 div:nth-child(3) a").unwrap();

    let name_selector = Selector::parse(".text-white .text-[14px] .font-bold .truncate").unwrap();
    let winrate_selector = Selector::parse(".text-[12px] .font-bold .leading-[15px] .whitespace-nowrap .text-right .text-accent-blue-400").unwrap();

    let best_picks = document
        .as_ref()
        .unwrap()
        .select(&best_picks_selector)
        .map(|anchor| {
            let name = anchor
                .select(&name_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("");
            let winrate = anchor
                .select(&winrate_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .expect("");
        })
        .collect::<Vec<String>>();

    let worst_picks = document
        .as_ref()
        .unwrap()
        .select(&worst_picks_selector)
        .map(|x| x.text().collect::<String>())
        .collect::<Vec<String>>();

    let lane_picks = document
        .as_ref()
        .unwrap()
        .select(&lane_picks_selector)
        .map(|x| x.text().collect::<String>())
        .collect::<Vec<String>>();

    Ok(DiscordOutput {
        title: format!("Best picks for {}", champion),
        description: best_picks.join("\n"),
        color: todo!(),
        fields: todo!(),
        footer: todo!(),
        content: todo!(),
    })
}

async fn get_u_gg_document_body(
    champion: &str,
    lane: Option<&str>,
    tab: &Arc<Tab>,
) -> Result<Html, Box<dyn std::error::Error>> {
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
