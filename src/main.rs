use anyhow::Context as _;
use serenity::all::*;
use shared::types::DiscordOutput;
use shuttle_runtime::SecretStore;
use tracing::info;

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
        let commands = vec![CreateCommand::new("matches")
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
            )];
        let commands = &self
            .discord_guild_id
            .set_commands(&ctx.http, commands)
            .await
            .unwrap();

        info!("Registered commands: {:#?}", commands);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let response_content: Result<DiscordOutput, Box<dyn std::error::Error>> =
                match command.data.name.as_str() {
                    "matches" => {
                        let (player_name, tag, region, game_count) = {
                            let iter = command.data.options.iter();
                            println!("{:?}", iter.clone());

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
                                    opt.value.as_i64().or_else(|| {
                                        opt.value.as_str().and_then(|s| s.parse::<i64>().ok())
                                    })
                                })
                                .unwrap_or_else(|| {
                                    println!("game_count not found or invalid, defaulting to 20");
                                    20
                                });
                            (player_name, tag, region, game_count)
                        };

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
                            Ok(matches_command_result) => Ok(matches_command_result),
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
                    command => unreachable!("Unknown command: {}", command),
                };

            let DiscordOutput {
                color,
                description,
                fields,
                footer,
                title,
                content,
            } = response_content.expect("");

            let data = CreateEmbed::new()
                .title(title)
                .description(description)
                .color(color)
                .fields(fields)
                .footer(footer);

            let builder = CreateMessage::new().content(content).embed(data);
            let channel_id = command.channel_id;
            let _ = channel_id.send_message(&ctx.http, builder).await;
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
