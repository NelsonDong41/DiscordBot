use anyhow::Context as _;
use serenity::all::*;
use shuttle_runtime::SecretStore;
use tracing::info;

mod league;
mod weather;

struct Bot {
    weather_api_key: String,
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
            CreateCommand::new("hello").description("Say hello"),
            CreateCommand::new("weather")
                .description("Display the weather")
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "place",
                        "City to lookup forecast",
                    )
                    .required(true),
                ),
            CreateCommand::new("league")
                .description("League info")
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "player_name",
                        "City to lookup forecast",
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
                        serenity::all::CommandOptionType::Number,
                        "game_count",
                        "Number of games to check",
                    )
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
            let response_content = match command.data.name.as_str() {
                "hello" => "hello".to_owned(),
                "weather" => {
                    let argument = command
                        .data
                        .options
                        .iter()
                        .find(|opt| opt.name == "place")
                        .cloned();
                    let value = argument.unwrap().value;
                    let place = value.as_str().unwrap();
                    let result =
                        weather::get_forecast(place, &self.weather_api_key, &self.client).await;
                    match result {
                        Ok((location, forecast)) => {
                            format!("Forecast: {} in {}", forecast.headline.overview, location)
                        }
                        Err(err) => {
                            format!("Err: {}", err)
                        }
                    }
                }
                "league" => {
                    let (player_name, tag, region, game_count) = {
                        let mut iter = command.data.options.iter();
                        let player_name = iter
                            .find(|opt| opt.name == "player_name")
                            .and_then(|opt| opt.value.as_str())
                            .unwrap();
                        let tag = iter
                            .find(|opt| opt.name == "tag")
                            .and_then(|opt| opt.value.as_str())
                            .unwrap();
                        let region = iter
                            .find(|opt| opt.name == "region")
                            .and_then(|opt| opt.value.as_str())
                            .unwrap_or("americas");
                        let game_count = iter
                            .find(|opt| opt.name == "game_count")
                            .and_then(|opt| opt.value.as_i64())
                            .unwrap_or(20);
                        (player_name, tag, region, game_count)
                    };
                    let result = league::get_league_info(
                        player_name,
                        tag,
                        region,
                        game_count,
                        &self.riot_api_key,
                        &self.client,
                    )
                    .await;
                    match result {
                        Ok(puuid) => {
                            format!("{}", puuid)
                        }
                        Err(err) => {
                            format!("Err: {}", err)
                        }
                    }
                }
                command => unreachable!("Unknown command: {}", command),
            };

            let data = CreateInteractionResponseMessage::new().content(response_content);
            let builder = CreateInteractionResponse::Message(data);

            if let Err(why) = command.create_response(&ctx.http, builder).await {
                println!("Cannot respond to slash command: {why}");
            }
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

    let weather_api_key = secret_store
        .get("WEATHER_API_KEY")
        .context("'WEATHER_API_KEY' was not found")?;

    let discord_guild_id = secret_store
        .get("DISCORD_GUILD_ID")
        .context("'DISCORD_GUILD_ID' was not found")?;

    let riot_api_key = secret_store
        .get("RIOT_API_KEY")
        .context("'RIOT_API_KEY' was not found")?;

    let client = get_client(
        &discord_token,
        &weather_api_key,
        discord_guild_id.parse().unwrap(),
        &riot_api_key,
    )
    .await;
    Ok(client.into())
}

pub async fn get_client(
    discord_token: &str,
    weather_api_key: &str,
    discord_guild_id: u64,
    riot_api_key: &str,
) -> Client {
    // Set gateway intents, which decides what events the bot will be notified about.
    // Here we don't need any intents so empty
    let intents = GatewayIntents::empty();

    Client::builder(discord_token, intents)
        .event_handler(Bot {
            weather_api_key: weather_api_key.to_owned(),
            client: reqwest::Client::new(),
            discord_guild_id: GuildId::new(discord_guild_id),
            riot_api_key: riot_api_key.to_owned(),
        })
        .await
        .expect("Err creating client")
}
