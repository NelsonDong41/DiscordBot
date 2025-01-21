use headless_chrome::Browser;
use scraper::{Html, Selector};
use serenity::all::{Color, CreateEmbedFooter};
use unicode_width::UnicodeWidthStr;

struct RuneBuildInfo {
    title: String,
    perks: Vec<Vec<bool>>,
}
struct RuneBuild {
    primary: RuneBuildInfo,
    secondary: RuneBuildInfo,
    shards: RuneBuildInfo,
}

use crate::shared::types::DiscordOutput;

const TRANSPARENT_CIRCLE: &str = "⚫";
const COLUMN_WIDTH: usize = 10;

pub async fn handle_build_command(
    champion1: &str,
    champion2: Option<&str>,
    lane: Option<&str>,
) -> Result<DiscordOutput, Box<dyn std::error::Error>> {
    let document = get_u_gg_document_body(champion1, champion2, lane).await?;

    let win_rate = get_winrate_as_f64(document.clone());
    let lane = get_lane(document.clone(), lane);

    let RuneBuild {
        primary,
        secondary,
        shards,
    } = get_runes(document);

    let primary_icon = get_color_from_rune_title(&primary.title).unwrap();
    let primary_tree = perks_to_colored_grid(primary.perks, primary_icon);
    let primary_tree_string_rows: Vec<String> = primary_tree.iter().map(grid_to_row).collect();
    let mut primary_tree_string_with_title = vec![primary.title];
    primary_tree_string_with_title.extend(primary_tree_string_rows);

    let secondary_icon = get_color_from_rune_title(&secondary.title).unwrap();
    let secondary_tree = perks_to_colored_grid(secondary.perks, secondary_icon);
    let secondary_tree_string_rows: Vec<String> = secondary_tree.iter().map(grid_to_row).collect();
    let mut secondary_tree_string_with_title = vec![secondary.title];
    secondary_tree_string_with_title.extend(secondary_tree_string_rows);

    let shards_tree = perks_to_colored_grid(shards.perks, "⚪");
    let shards_tree_string_rows: Vec<String> = shards_tree.iter().map(grid_to_row).collect();
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

    return Ok(DiscordOutput::new(
        color,
        description,
        vec![rune_field],
        CreateEmbedFooter::new(""),
        title,
        "".to_string(),
    ));
}

async fn get_u_gg_document_body(
    champion1: &str,
    champion2: Option<&str>,
    lane: Option<&str>,
) -> Result<Html, Box<dyn std::error::Error>> {
    let mut u_gg_url = format!("https://u.gg/lol/champions/{}/build", champion1);
    match lane {
        Some(x) => u_gg_url = u_gg_url + format!("/{}", x).as_str(),
        _ => {}
    }

    match champion2 {
        Some(x) => u_gg_url = u_gg_url + format!("?opp={}", x).as_str(),
        _ => {}
    }

    let browser = Browser::default()?;

    let tab = browser.new_tab()?;

    tab.navigate_to(&u_gg_url)?;

    tab.wait_for_element("div.champion-recommended-build")?;
    tab.wait_for_element("a.role-filter.active")?;

    let champion_information = tab.find_element("div.champion-recommended-build")?;

    Ok(Html::parse_document(&champion_information.get_content()?))
}

fn get_winrate_as_f64(document: Html) -> f64 {
    let winrate_selector = Selector::parse(
        "div:first-child > div:first-child > div:first-child > div:first-child > div:first-child",
    )
    .unwrap();

    let mut win_rate_string = document
        .select(&winrate_selector)
        .next()
        .expect("")
        .text()
        .collect::<String>()
        .trim()
        .to_string();

    win_rate_string.pop();
    win_rate_string.parse::<f64>().unwrap_or(0.0)
}

fn get_lane(document: Html, lane: Option<&str>) -> String {
    match lane {
        Some(x) => x.to_string().to_uppercase(),
        None => {
            let lane_selector = Selector::parse("a.role-filter.active").unwrap();
            document
                .select(&lane_selector)
                .next()
                .expect("")
                .text()
                .collect::<String>()
                .trim()
                .to_uppercase()
        }
    }
}

fn get_runes(document: Html) -> RuneBuild {
    let primary_rune_title_selector =
        Selector::parse("div.media-query_MOBILE_LARGE__DESKTOP_LARGE div.rune-tree.primary-tree div.perk-style-title div.pointer").unwrap();
    let primary_rune_tree_selector = Selector::parse(
        "div.media-query_MOBILE_LARGE__DESKTOP_LARGE div.rune-tree.primary-tree div.perk-row div.perks",
    )
    .unwrap();
    let secondary_rune_title_selector =
        Selector::parse("div.media-query_MOBILE_LARGE__DESKTOP_LARGE div.secondary-tree div.perk-style-title div.pointer").unwrap();
    let secondary_rune_tree_selector =
        Selector::parse("div.media-query_MOBILE_LARGE__DESKTOP_LARGE div.secondary-tree :first-child div.rune-tree div.perk-row div.perks").unwrap();
    let stat_shard_selector = Selector::parse(
        "div.media-query_MOBILE_LARGE__DESKTOP_LARGE div.stat-shards-container div.perk-row div.perks",
    )
    .unwrap();

    let primary_rune_title = document
        .clone()
        .select(&primary_rune_title_selector)
        .next()
        .expect("")
        .text()
        .collect::<String>();

    let primary_runes: Vec<Vec<bool>> = document
        .clone()
        .select(&primary_rune_tree_selector)
        .map(|row| {
            row.child_elements()
                .map(|child| child.attr("class").unwrap().contains("perk-active"))
                .collect()
        })
        .collect();

    let secondary_rune_title = document
        .clone()
        .select(&secondary_rune_title_selector)
        .next()
        .expect("")
        .text()
        .collect::<String>();

    let secondary_runes: Vec<Vec<bool>> = document
        .clone()
        .select(&secondary_rune_tree_selector)
        .map(|row| {
            row.child_elements()
                .map(|child| child.attr("class").unwrap().contains("perk-active"))
                .collect()
        })
        .collect();

    let stat_shards: Vec<Vec<bool>> = document
        .clone()
        .select(&stat_shard_selector)
        .map(|row| {
            row.child_elements()
                .map(|child| child.attr("class").unwrap().contains("shard-active"))
                .collect()
        })
        .collect();

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

fn get_descriptors(win_rate: f64) -> (Color, String) {
    if win_rate > 50.0 {
        return (Color::BLUE, format!("You better win ({}%)", win_rate));
    }
    if win_rate > 48.0 {
        return (Color::DARK_GOLD, format!("Hmm ({}%)", win_rate));
    }
    return (Color::DARK_RED, format!("You gon lose ({}%)", win_rate));
}

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

fn capitalize_string(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

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

        let left = val1.unwrap_or_default();
        let right = val2.unwrap_or_default();

        let left_width = left.width();

        let left_padding = " ".repeat(COLUMN_WIDTH.saturating_sub(left_width));

        acc.push_str(&format!("{}{}{}\n", left, left_padding, right));
    }

    println!("Columned:\n{}", acc);
    acc
}

fn grid_to_row(row: &Vec<&str>) -> String {
    let mut result: String = String::new();
    result = row
        .iter()
        .fold(result, |acc2, cell| format!("{}{}", acc2, cell));
    result
}
