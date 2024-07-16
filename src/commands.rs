use crate::{Context, Error};
use ::serenity::all::{CreateEmbedAuthor, CreateEmbedFooter};
use poise::{serenity_prelude as serenity, CreateReply};

struct Fumo {
    id: u64,
    caption: Option<String>,
    image: String,
    source: Option<String>,
    credit: Option<String>,
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
    let response = CreateReply::default().embed(
        serenity::CreateEmbed::new()
            .title("Fumo [#ID]")
            .description("[Caption if present]")
            .image("[url]")
            .footer(CreateEmbedFooter::new("Source: [source]"))
            .author(CreateEmbedAuthor::new("[credit]")),
    );
    ctx.send(response).await?;
    todo!("Retrieve fumo from the fumo-API");
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn random(ctx: Context<'_>) -> Result<(), Error> {
    let response = CreateReply::default().embed(
        serenity::CreateEmbed::new()
            .title("Fumo [#ID]")
            .description("[Caption if present]")
            .image("[url]")
            .footer(CreateEmbedFooter::new("Source: [source]"))
            .author(CreateEmbedAuthor::new("[credit]")),
    );
    ctx.send(response).await?;
    todo!("Retrieve random fumo from the fumo-API");
    Ok(())
}

pub fn generate_fumo_embed(fumo: Fumo) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new()
        .title(format!("Fumo #{}", fumo.id))
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
