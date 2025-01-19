use headless_chrome::Browser;
use html5ever::serialize::{serialize, SerializeOpts};
use scraper::{Html, Selector};
use serenity::all::{Color, CreateEmbedFooter};

struct RuneBuildInfo {
    title: String,
    perks: Vec<usize>,
}
struct RuneBuild {
    primary: RuneBuildInfo,
    secondary: RuneBuildInfo,
    shards: RuneBuildInfo,
}

use crate::shared::types::DiscordOutput;

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

    let rune_field: (String, String, bool) = (
        "Runes".to_string(),
        format!(
            "```\n{}\n{:?}\n{}\n{:?}\n{}\n{:?}```",
            primary.title,
            primary.perks,
            secondary.title,
            secondary.perks,
            shards.title,
            shards.perks
        ),
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
        Selector::parse("div.rune-tree.primary-tree div.perk-style-title div.pointer").unwrap();
    let primary_rune_tree_selector =
        Selector::parse("div.rune-tree.primary-tree div.perk-row div.perks").unwrap();
    let secondary_rune_title_selector =
        Selector::parse("div.secondary-tree div.perk-style-title div.pointer").unwrap();
    let secondary_rune_tree_selector =
        Selector::parse("div.secondary-tree div.rune-tree div.perk-row div.perks").unwrap();
    let stat_shard_selector =
        Selector::parse("div.stat-shards-container div.perk-row div.perks").unwrap();

    let primary_rune_title = document
        .clone()
        .select(&primary_rune_title_selector)
        .next()
        .expect("")
        .text()
        .collect::<String>();

    println!("primary title {}", primary_rune_title);

    let primary_runes = document
        .clone()
        .select(&primary_rune_tree_selector)
        .inspect(|x| println!("what the {:?}", x))
        .map(|row| {
            row.child_elements()
                .position(|child| {
                    println!("{}", child.inner_html());
                    child.attr("class").unwrap().contains("perk-active")
                })
                .expect("")
        })
        .collect::<Vec<usize>>();
    println!("primary runes {:?}", primary_runes);

    let secondary_rune_title = document
        .clone()
        .select(&secondary_rune_title_selector)
        .next()
        .expect("")
        .text()
        .collect::<String>();

    let secondary_runes = document
        .clone()
        .select(&secondary_rune_tree_selector)
        .inspect(|x| println!("SIZE OF SECONDARY RUN ROWS {}", x.child_elements().count()))
        .map(|row| {
            row.child_elements()
                .position(|child| child.attr("class").unwrap().contains("perk-active"))
                .unwrap_or(usize::MAX)
        })
        .collect::<Vec<usize>>();

    let stat_shards = document
        .clone()
        .select(&stat_shard_selector)
        .map(|row| {
            row.child_elements()
                .position(|child| child.attr("class").unwrap().contains("shard-active"))
                .unwrap()
        })
        .collect::<Vec<usize>>();

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
