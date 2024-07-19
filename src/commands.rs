use crate::{Context, Error};
use ::serenity::all::{CreateEmbedAuthor, CreateEmbedFooter};
use poise::{serenity_prelude as serenity, CreateReply};
use serde;
use serde::{Deserialize, Serialize};
use serde_json;

pub struct Fumo {
    pub _id: String,
    pub caption: Option<String>,
    pub image: String,
    pub source: Option<String>,
    pub credit: Option<String>,
    pub featured: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct APIFumo {
    pub _id: String,
    pub caption: Option<String>,
    pub url: String,
    pub source: Option<String>,
    pub credit: Option<String>,
    pub featured: Option<String>,
}

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is a bot designed to help curating the fumoApi and retrive fumo images from the FUMO-API",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn hello(
    ctx: Context<'_>,
    #[description = "The name of the person you want to greet"] who: Option<serenity::User>,
) -> Result<(), Error> {
    if who.is_some() {
        let response = format!(
            "Hello {}! {} is greeting you",
            who.unwrap().name,
            ctx.author().name
        );
        ctx.say(response).await?;
        return Ok(());
    }
    let response = format!("Hello chat how you doin!");
    ctx.say(response).await?;
    Ok(())
}
#[poise::command(prefix_command, slash_command)]
pub async fn fumo(
    ctx: Context<'_>,
    #[description = "The id of the fumo you want to search for"] fumo: String,
) -> Result<(), Error> {
    let data = ctx.data();
    let client = data.web_client.clone();

    let res = client.get(format!("{}/fumo/{}", data.fumo_api_endpoint, fumo));
    let res = res.send().await.expect("Failed to retrieve fumo");

    let fumo: APIFumo = res.json().await.expect("Failed to parse fumo");

    println!("{:?}", fumo.url.to_string());

    let fumo = Fumo {
        _id: fumo._id.to_string(),
        caption: fumo.caption,
        image: fumo.url,
        source: fumo.source,
        credit: fumo.credit,
        featured: Some("Reimu".to_string()),
    };
    let embed = generate_fumo_embed(fumo);
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn random(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let client = data.web_client.clone();

    let res = client.get(format!("{}/random", data.fumo_api_endpoint));
    let res = res.send().await.expect("Failed to retrieve fumo");

    let fumo: APIFumo = res.json().await.expect("Failed to parse fumo");

    let fumo = Fumo {
        _id: fumo._id,
        caption: fumo.caption,
        image: fumo.url,
        source: fumo.source,
        credit: fumo.credit,
        featured: Some("Reimu".to_string()),
    };
    let embed = generate_fumo_embed(fumo);
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

pub fn generate_fumo_embed(fumo: Fumo) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new()
        .title(format!("Fumo #{}", fumo._id))
        .description(fumo.caption.unwrap_or_else(|| "No caption".to_string()))
        .image(fumo.image)
        .footer(CreateEmbedFooter::new(format!(
            "Source: {}",
            fumo.source.unwrap_or_else(|| "Unknown".to_string())
        )))
        .author(CreateEmbedAuthor::new(
            fumo.credit.unwrap_or_else(|| "Unknown".to_string()),
        ))
}
