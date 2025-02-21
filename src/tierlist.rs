use std::sync::Arc;

use headless_chrome::Tab;
use scraper::{Html, Selector};
use serenity::all::Colour;

use crate::shared::types::DiscordOutput;

pub async fn handle_tierlist_command(
    lane: Option<&str>,
    tab: &Arc<Tab>,
    count: usize,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    let url = format!(
        "https://u.gg/lol/{}tier-list",
        if let Some(lane) = lane {
            if lane == "top" {
                "top-lane-".to_string()
            } else {
                format!("{}-", lane)
            }
        } else {
            "".to_string()
        }
    );
    let table_row_selector = "#content .tier-list .rt-tbody";

    tab.navigate_to(&url)?;
    tab.wait_until_navigated()?;

    let row = tab.wait_for_element(table_row_selector)?;
    let document = Html::parse_document(&row.get_content()?);

    let tier_list = document
        .select(&Selector::parse(".rt-tr-group .rt-tr").unwrap())
        .take(count)
        .map(|row| {
            let name = row
                .select(&Selector::parse(".rt-td:nth-child(3) strong").unwrap())
                .next()
                .unwrap()
                .text()
                .next()
                .unwrap()
                .trim();
            let winrate = row
                .select(&Selector::parse(".rt-td:nth-child(5) b").unwrap())
                .next()
                .unwrap()
                .text()
                .next()
                .unwrap()
                .trim();
            format!("{:<20} - {:<20}", name, winrate)
        })
        .collect::<Vec<String>>()
        .join("\n");

    Ok(DiscordOutput {
        title: format!("Top {} tier list for {}", count, lane.unwrap_or("All")),
        description: "".to_string(),
        color: Colour::DARK_GREEN,
        fields: vec![(
            "Champion - Tier".to_string(),
            format!("```{}```", tier_list),
            false,
        )],
        footer: "".to_string(),
        content: "".to_string(),
    })
}
