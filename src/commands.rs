use crate::{Context, Error};
use poise::serenity_prelude as serenity;

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
