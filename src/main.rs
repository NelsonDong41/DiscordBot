use anyhow::Context as _;
use serenity::all::*;
use shared::types::DiscordOutput;
use shuttle_runtime::SecretStore;
use tracing::info;

mod build;
mod matches;
pub mod shared;

struct Bot {
    client: reqwest::Client,
    discord_guild_id: GuildId,
    riot_api_key: String,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // We are creating a vector with commands
        // and registering them on the server with the guild ID we have set.
        let commands = vec![
            CreateCommand::new("matches")
                .description("Get match info for player")
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "player_name",
                        "Player Name",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "tag",
                        "playerTag",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "region",
                        "Region",
                    )
                    .required(false),
                )
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::Integer,
                        "game_count",
                        "Number of games to check",
                    )
                    .min_int_value(0)
                    .max_int_value(40)
                    .required(false),
                ),
            CreateCommand::new("john").description("Look at this guy"),
            CreateCommand::new("build")
                .description("Get build data")
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "you",
                        "Your champ",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "enemy",
                        "Enemy Champ",
                    )
                    .required(false),
                )
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "lane",
                        "Lane",
                    )
                    .add_string_choice("top", "Top")
                    .add_string_choice("mid", "Mid")
                    .add_string_choice("jungle", "Jungle")
                    .add_string_choice("adc", "Adc")
                    .add_string_choice("support", "Support")
                    .required(false),
                ),
        ];
        let commands = &self
            .discord_guild_id
            .set_commands(&ctx.http, commands)
            .await
            .unwrap();

        info!("Registered commands: {:#?}", commands);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let builder = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
            command.create_response(&ctx.http, builder).await.unwrap();

            let response_content: DiscordOutput = match command.data.name.as_str() {
                "matches" => {
                    let iter = command.data.options.iter();

                    let player_name = iter
                        .clone()
                        .find(|opt| opt.name == "player_name")
                        .and_then(|opt| opt.value.as_str())
                        .unwrap();
                    let tag = iter
                        .clone()
                        .find(|opt| opt.name == "tag")
                        .and_then(|opt| opt.value.as_str())
                        .unwrap();
                    let region = iter
                        .clone()
                        .find(|opt| opt.name == "region")
                        .and_then(|opt| opt.value.as_str())
                        .unwrap_or("americas");
                    let game_count = iter
                        .clone()
                        .find(|opt| opt.name == "game_count")
                        .and_then(|opt| {
                            opt.value
                                .as_i64()
                                .or_else(|| opt.value.as_str().and_then(|s| s.parse::<i64>().ok()))
                        })
                        .unwrap_or_else(|| {
                            println!("game_count not found or invalid, defaulting to 20");
                            20
                        });

                    let matches_command_result = matches::handle_matches_command(
                        player_name,
                        tag,
                        region,
                        game_count,
                        &self.riot_api_key,
                        &self.client,
                    )
                    .await;
                    match matches_command_result {
                        Ok(matches_command_result) => {
                            Ok::<DiscordOutput, Error>(matches_command_result)
                        }
                        Err(err) => {
                            println!("Error: {}", err);
                            Ok(DiscordOutput::new(
                                Colour::RED,
                                "".to_string(),
                                vec![],
                                CreateEmbedFooter::new(err.to_string()),
                                format!("Request for {}'s matches FAILED", player_name),
                                "".to_string(),
                            ))
                        }
                    }
                }
                "john" => {
                    let matches_command_result = matches::handle_matches_command(
                        "SolarKnight0",
                        "NA2",
                        "Americas",
                        20,
                        &self.riot_api_key,
                        &self.client,
                    )
                    .await;
                    match matches_command_result {
                        Ok(matches_command_result) => Ok(matches_command_result),
                        Err(err) => {
                            println!("Error: {}", err);
                            Ok(DiscordOutput::new(
                                Colour::RED,
                                "".to_string(),
                                vec![],
                                CreateEmbedFooter::new(err.to_string()),
                                format!("Request for {}'s matches FAILED", "SolarKnight0"),
                                "".to_string(),
                            ))
                        }
                    }
                }
                "build" => {
                    let iter = command.data.options.iter();

                    let you = iter
                        .clone()
                        .find(|opt| opt.name == "you")
                        .and_then(|opt| opt.value.as_str())
                        .unwrap();
                    let enemy_arg = iter.clone().find(|opt| opt.name == "enemy");

                    let enemy = match enemy_arg {
                        Some(enemy) => enemy.value.as_str(),
                        None => None,
                    };

                    let lane_arg = iter.clone().find(|opt| opt.name == "lane");
                    let lane: Option<&str> = match lane_arg {
                        Some(lane) => lane.value.as_str(),
                        None => None,
                    };

                    let build_command_result = build::handle_build_command(you, enemy, lane).await;

                    match build_command_result {
                        Ok(build_command_result) => Ok(build_command_result),
                        Err(err) => {
                            println!("Error: {}", err);
                            Ok(DiscordOutput::new(
                                Colour::RED,
                                "".to_string(),
                                vec![],
                                CreateEmbedFooter::new(err.to_string()),
                                format!("Request for {}'s matches FAILED", "SolarKnight0"),
                                "".to_string(),
                            ))
                        }
                    }
                }
                command => unreachable!("Unknown command: {}", command),
            }
            .expect("");

            let DiscordOutput {
                color,
                description,
                fields,
                footer,
                title,
                content,
            } = response_content;

            let data = CreateEmbed::new()
                .title(title)
                .description(description)
                .color(color)
                .fields(fields)
                .footer(footer);

            let edit_builder = EditInteractionResponse::new().content(content).embed(data);
            command
                .edit_response(&ctx.http, edit_builder)
                .await
                .unwrap();
        }
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let discord_guild_id = secret_store
        .get("DISCORD_GUILD_ID")
        .context("'DISCORD_GUILD_ID' was not found")?;

    let riot_api_key = secret_store
        .get("RIOT_API_KEY")
        .context("'RIOT_API_KEY' was not found")?;

    let client = get_client(
        &discord_token,
        discord_guild_id.parse().unwrap(),
        &riot_api_key,
    )
    .await;
    Ok(client.into())
}

pub async fn get_client(discord_token: &str, discord_guild_id: u64, riot_api_key: &str) -> Client {
    // Set gateway intents, which decides what events the bot will be notified about.
    // Here we don't need any intents so empty
    let intents = GatewayIntents::empty();

    Client::builder(discord_token, intents)
        .event_handler(Bot {
            client: reqwest::Client::new(),
            discord_guild_id: GuildId::new(discord_guild_id),
            riot_api_key: riot_api_key.to_owned(),
        })
        .await
        .expect("Err creating client")
}
