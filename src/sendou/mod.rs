mod schema;

use crate::sendou::schema::TournamentRoot;
use crate::{Error, Result};
use reqwest::Url;
use serde::Deserialize;
use serenity::all::{Builder, Channel, ChannelType, HttpBuilder};
use std::path::Path;
use std::str::FromStr;

pub async fn sendou_cli(in_db: &Path, out_db: &Path, tournament_url: &str) -> Result<()> {
    let tournament_url = {
        let mut url = Url::parse(tournament_url)?;
        url.set_query(Some("_data=features/tournament/routes/to.$id"));
        url
    };
    let client = reqwest::ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            " (https://github.com/Gaming32/switzerland-power-calc, ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .build()?;

    let http = HttpBuilder::new(env_str("DISCORD_BOT_TOKEN")?)
        .client(client)
        .build();

    let chat_category = match http.get_channel(env("DISCORD_CHAT_CATEGORY_ID")?).await? {
        Channel::Private(channel) => {
            return Err(Error::Custom(format!(
                "Discord channel {} is not part of a guild",
                channel.name()
            )));
        }
        Channel::Guild(channel) if channel.kind != ChannelType::Category => {
            return Err(Error::Custom(format!(
                "Discord channel {} is not a Category channel, but a {:?} channel",
                channel.name, channel.kind
            )));
        }
        Channel::Guild(category) => category,
        _ => return Err(Error::Custom("Your Discord channel is weird".to_string())),
    };

    let base_tournament = client
        .get(tournament_url)
        .send()
        .await?
        .json::<TournamentRoot>()
        .await?
        .tournament
        .context;

    // CreateMessage::new()
    //     .content("This is a test message")
    //     .execute(&http, (ChannelId::new(738850747071987794), None))
    //     .await
    //     .unwrap();

    Ok(())
}

fn env_str(var: &str) -> Result<String> {
    dotenvy::var(var).map_err(|_| Error::MissingEnv(var.to_string()))
}

fn env<T: FromStr>(var: &str) -> Result<T>
where
    <T as FromStr>::Err: std::error::Error + 'static,
{
    env_str(var)?
        .parse()
        .map_err(|e| Error::InvalidEnv(var.to_string(), Box::new(e)))
}
