use std::{sync::Arc, vec};

use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use scraper::Html;
use scraper::Selector;
use serenity::all::{Color, Colour};
use unicode_width::UnicodeWidthStr;

use tracing::info;
use tracing::instrument;

struct RuneBuildInfo {
    title: String,
    perks: Vec<Vec<bool>>,
}
struct RuneBuild {
    primary: RuneBuildInfo,
    secondary: RuneBuildInfo,
    shards: RuneBuildInfo,
}
type ItemBuildInfo = Vec<(String, String, bool)>;

use crate::shared::types::DiscordOutput;

const TRANSPARENT_CIRCLE: &str = "⚫";
const COLUMN_WIDTH: usize = 15;

#[instrument(fields(champion1 = champion1, champion2 = champion2, lane = lane))]
pub async fn handle_build_command(
    champion1: &str,
    champion2: Option<&str>,
    lane: Option<&str>,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    // Your existing code here
    let browser = Browser::new(LaunchOptionsBuilder::default().headless(true).build()?)?;
    let tab = browser.new_tab()?;

    let document = get_u_gg_document_body(champion1, champion2, lane, &tab).await;

    // Match document
    match document {
        Err(err) => {
            return Ok(DiscordOutput::new(
                Colour::DARK_RED,
                err.to_string(),
                vec![],
                "".to_string(),
                format!("Error fetching build for {}", champion1),
                "".to_string(),
            ));
        }
        Ok(document) => {
            let win_rate = get_winrate_as_f64(&document, &champion2);
            let lane = get_lane(&tab, lane);

            let RuneBuild {
                primary,
                secondary,
                shards,
            } = get_runes(&tab);

            let primary_icon = get_color_from_rune_title(&primary.title).unwrap();
            let primary_tree = perks_to_colored_grid(primary.perks, primary_icon);
            let primary_tree_string_rows: Vec<String> =
                primary_tree.iter().map(grid_to_row).collect();
            let mut primary_tree_string_with_title = vec![primary.title];
            primary_tree_string_with_title.extend(primary_tree_string_rows);

            let secondary_icon = get_color_from_rune_title(&secondary.title).unwrap();
            let secondary_tree = perks_to_colored_grid(secondary.perks, secondary_icon);
            let secondary_tree_string_rows: Vec<String> =
                secondary_tree.iter().map(grid_to_row).collect();
            let mut secondary_tree_string_with_title = vec![secondary.title];
            secondary_tree_string_with_title.extend(secondary_tree_string_rows);

            let shards_tree = perks_to_colored_grid(shards.perks, "⚪");
            let shards_tree_string_rows: Vec<String> =
                shards_tree.iter().map(grid_to_row).collect();
            let mut shards_tree_string_with_title = vec![shards.title];
            shards_tree_string_with_title.extend(shards_tree_string_rows);

            secondary_tree_string_with_title.extend(shards_tree_string_with_title);

            let zipped_to_columns = columnize_trees(
                primary_tree_string_with_title.iter(),
                secondary_tree_string_with_title.iter(),
            );

            let rune_field: (String, String, bool) = (
                "Runes".to_string(),
                format!("```{}```", zipped_to_columns),
                false,
            );
            let (color, description) = get_descriptors(win_rate);
            let title = get_title(champion1, champion2, &lane);

            let item_build_info = generate_item_build_info(&tab).expect("");

            let mut fields = vec![rune_field];
            fields.extend_from_slice(&item_build_info);

            return Ok(DiscordOutput::new(
                color,
                description,
                fields,
                "".to_string(),
                title,
                "".to_string(),
            ));
        }
    }
}

#[instrument(skip(tab), fields(champion1 = champion1, champion2 = champion2, lane = lane))]
async fn get_u_gg_document_body(
    champion1: &str,
    champion2: Option<&str>,
    lane: Option<&str>,
    tab: &Arc<Tab>,
) -> Result<Html, Box<dyn std::error::Error>> {
    let no_data_found_selector =
        ".flex .items-center .flex-col .w-full .py-[60px] .px-[12px] .bg-purple-400 .rounded-[3px]";
    let mut u_gg_url = String::with_capacity(128);

    u_gg_url.push_str("https://u.gg/lol/champions/");
    u_gg_url.push_str(champion1);
    u_gg_url.push_str("/build");

    if let Some(x) = lane {
        u_gg_url.push_str("/");
        u_gg_url.push_str(x);
    }
    if let Some(x) = champion2 {
        u_gg_url.push_str("?opp=");
        u_gg_url.push_str(x);
    }

    tab.navigate_to(&u_gg_url)?;
    tab.wait_until_navigated()?;

    if tab.wait_for_element(no_data_found_selector).is_ok() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "No data found for {} vs. {}",
                champion1,
                champion2.unwrap_or(""),
            ),
        )));
    }

    tab.wait_for_element(".champion-recommended-build")?;
    tab.wait_for_element("a.role-filter.active")?;

    let champion_information = tab.find_element("div.champion-recommended-build")?;

    Ok(Html::parse_document(&champion_information.get_content()?))
}

#[instrument(skip(document), fields(enemy = enemy,))]
fn get_winrate_as_f64(document: &Html, enemy: &Option<&str>) -> f64 {
    let winrate_selector = Selector::parse(".champion-recommended-build > div:first-child > div:first-child > div:first-child > div:first-child").unwrap();
    let winrate_selector_if_opp = Selector::parse(".champion-recommended-build > div:first-child > div:first-child > div:nth-child(2) > div:first-child").unwrap();

    let mut win_rate_string: String = if enemy.is_some() {
        document
            .select(&winrate_selector)
            .next()
            .expect("")
            .text()
            .next()
            .expect("")
            .to_string()
    } else {
        document
            .select(&winrate_selector_if_opp)
            .next()
            .expect("")
            .text()
            .next()
            .expect("")
            .to_string()
    };

    win_rate_string.pop();
    win_rate_string.parse::<f64>().unwrap_or(0.0)
}

#[instrument(skip(tab, lane), fields(lane = lane))]
fn get_lane(tab: &Arc<Tab>, lane: Option<&str>) -> String {
    match lane {
        Some(x) => x.to_string().to_uppercase(),
        None => {
            let lane_selector =
                ".media-query_MOBILE_SMALL__DESKTOP_SMALL .filter-select .role-value div";
            tab.wait_for_element(&lane_selector)
                .expect("")
                .get_inner_text()
                .expect("")
                .to_uppercase()
        }
    }
}

#[instrument(skip(tab))]
fn get_runes(tab: &Arc<Tab>) -> RuneBuild {
    let primary_rune_title_selector =
        ".media-query_MOBILE_LARGE__DESKTOP_LARGE .rune-tree.primary-tree .perk-style-title .pointer";
    let secondary_rune_title_selector =
        ".media-query_MOBILE_LARGE__DESKTOP_LARGE .secondary-tree .perk-style-title .pointer";

    // Combined selectors for efficiency
    let primary_rune_selector =
        ".media-query_MOBILE_LARGE__DESKTOP_LARGE .rune-tree.primary-tree .perk-row .perks .perk";
    let secondary_rune_selector =
        ".media-query_MOBILE_LARGE__DESKTOP_LARGE .secondary-tree :first-child .rune-tree .perk-row .perks .perk";
    let stat_shard_selector =
        ".media-query_MOBILE_LARGE__DESKTOP_LARGE .stat-shards-container .perk-row .perks .shard";

    let primary_rune_title = tab
        .wait_for_element(&primary_rune_title_selector)
        .expect("")
        .get_inner_text()
        .expect("");

    let primary_runes = tab
        .wait_for_elements(&primary_rune_selector)
        .expect("")
        .into_iter()
        .map(|child| {
            child
                .get_attribute_value("class")
                .unwrap()
                .unwrap()
                .contains("perk-active")
        })
        .collect::<Vec<bool>>()
        .into_iter()
        .enumerate()
        .fold(Vec::new(), |mut acc, (i, value)| {
            if i == 0 {
                acc.push(Vec::new());
            } else if i == 4 || (i > 4 && (i - 4) % 3 == 0) {
                acc.push(Vec::new());
            }
            acc.last_mut().unwrap().push(value);
            acc
        });

    let secondary_rune_title = tab
        .wait_for_element(&secondary_rune_title_selector)
        .expect("")
        .get_inner_text()
        .expect("");

    let secondary_runes: Vec<Vec<bool>> = tab
        .wait_for_elements(&secondary_rune_selector)
        .expect("Failed to find secondary rune elements")
        .into_iter()
        .map(|child| {
            child
                .get_attribute_value("class")
                .unwrap()
                .unwrap()
                .contains("perk-active")
        })
        .collect::<Vec<bool>>()
        .chunks(3)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<Vec<bool>>>();

    let stat_shards = tab
        .wait_for_elements(&stat_shard_selector)
        .expect("")
        .into_iter()
        .map(|child| {
            child
                .get_attribute_value("class")
                .unwrap()
                .unwrap()
                .contains("shard-active")
        })
        .collect::<Vec<bool>>()
        .chunks(3) //Chunk into rows of 3
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<Vec<bool>>>();

    return RuneBuild {
        primary: RuneBuildInfo {
            title: primary_rune_title,
            perks: primary_runes,
        },
        secondary: RuneBuildInfo {
            title: secondary_rune_title,
            perks: secondary_runes,
        },
        shards: RuneBuildInfo {
            title: "Stat Shards".to_string(),
            perks: stat_shards,
        },
    };
}

#[instrument(fields(win_rate = win_rate))]
fn get_descriptors(win_rate: f64) -> (Color, String) {
    if win_rate > 50.0 {
        return (Color::BLUE, format!("You better win ({}%)", win_rate));
    }
    if win_rate > 48.0 {
        return (Color::DARK_GOLD, format!("Hmm ({}%)", win_rate));
    }
    return (Color::DARK_RED, format!("You gon lose ({}%)", win_rate));
}

#[instrument(fields(champion1 = champion1, champion2 = champion2, lane = lane))]
fn get_title(champion1: &str, champion2: Option<&str>, lane: &str) -> String {
    match champion2 {
        Some(enemy_champ) => {
            return format!(
                "({}) {} vs. {}",
                lane,
                capitalize_string(champion1),
                capitalize_string(enemy_champ)
            )
        }
        None => return format!("({}) {}", lane, champion1),
    }
}

#[instrument(fields(input = input))]
fn capitalize_string(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[instrument(fields(title = title))]
fn get_color_from_rune_title(title: &str) -> Result<&str, Box<dyn std::error::Error>> {
    match title {
        "Precision" => Ok("🟡"),
        "Resolve" => Ok("🟢"),
        "Inspiration" => Ok("🔵"),
        "Domination" => Ok("🔴"),
        "Sorcery" => Ok("🟣"),
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "TItle doesn't map to a color",
        ))),
    }
}

#[instrument(fields(icon = icon))]
fn perks_to_colored_grid(grid: Vec<Vec<bool>>, icon: &str) -> Vec<Vec<&str>> {
    grid.iter()
        .map(|row| {
            row.iter()
                .map(|cell| if *cell { icon } else { TRANSPARENT_CIRCLE })
                .collect()
        })
        .collect()
}

fn columnize_trees<'a, T: Iterator<Item = &'a String>>(iter1: T, iter2: T) -> String {
    let mut acc = String::new();
    let mut iter1clone = iter1.cloned();
    let mut iter2clone = iter2.cloned();

    loop {
        let val1 = iter1clone.next();
        let val2 = iter2clone.next();

        if val1.is_none() && val2.is_none() {
            break;
        }

        let left = val1.unwrap_or(String::new());
        let right = val2.unwrap_or(String::new());

        let left_width = left.width();

        let left_padding = " ".repeat(COLUMN_WIDTH.saturating_sub(left_width));

        acc.push_str(&format!("{}{}{}\n", left, left_padding, right));
    }

    acc
}

#[instrument]
fn grid_to_row(row: &Vec<&str>) -> String {
    let mut result: String = String::new();
    result = row
        .iter()
        .fold(result, |acc2, cell| format!("{}{}", acc2, cell));
    result
}

#[instrument(skip(tab))]
fn generate_item_build_info(tab: &Arc<Tab>) -> Result<ItemBuildInfo, Box<dyn std::error::Error>> {
    let starting_items_selector = ".recommended-build_items .starting-items .item-img";
    let core_items_selector = ".recommended-build_items .core-items .image-wrapper";
    let fourth_item_options_selector = ".recommended-build_items .item-options-1 .item-img";
    let fifth_item_options_selector = ".recommended-build_items .item-options-2 .item-img";
    let sixth_item_options_selector = ".recommended-build_items .item-options-3 .item-img";

    let starting_items = find_names_for_items(&tab, starting_items_selector)?.join(" > ");
    let core_items = find_names_for_items(&tab, core_items_selector)?.join(" > ");
    let fourth_item_options = find_names_for_items(&tab, fourth_item_options_selector)?.join(" > ");
    let fifth_item_options = find_names_for_items(&tab, fifth_item_options_selector)?.join(" > ");
    let sixth_item_options = find_names_for_items(&tab, sixth_item_options_selector)?.join(" > ");

    Ok(vec![
        ("Starting Items".to_string(), starting_items, false),
        ("Core Items".to_string(), core_items, false),
        (
            "Fourth Item Options".to_string(),
            fourth_item_options,
            false,
        ),
        ("Fifth Item Options".to_string(), fifth_item_options, false),
        ("Sixth Item Options".to_string(), sixth_item_options, false),
    ])
}

#[instrument(skip(tab))]
fn find_names_for_items(
    tab: &Arc<Tab>,
    selector: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let tooltip_text_selector = "div#tooltip-portal .tooltip-item .name";
    let elements = tab.wait_for_elements(selector)?;
    let mut item_names = Vec::new();

    for (_index, element) in elements.iter().enumerate() {
        element.move_mouse_over()?;

        let tooltip_element = tab.wait_for_element(tooltip_text_selector)?;

        let tooltip_text = tooltip_element
            .get_inner_text()
            .expect("Failed to get tooltip text");
        item_names.push(tooltip_text);
    }
    Ok(item_names)
}
