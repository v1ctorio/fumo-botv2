#![warn(clippy::str_to_string)]

mod commands;

use ::serenity::all::{
    ChannelId, ComponentType, CreateButton, CreateInteractionResponseMessage, CreateMessage, UserId,
};
use dotenv::dotenv;
use env_logger;
use lazy_static::lazy_static;
use poise::serenity_prelude as serenity;
use std::{env::var, sync::Arc, time::Duration};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    //    votes: Mutex<HashMap<String, u32>>,
}

#[derive(Debug, poise::Modal)]
struct MoreInfoModal {
    credit: Option<String>,
    source: Option<String>,
    caption: Option<String>,
    featured: Option<String>, // plushie featured in the image
}

lazy_static! {
    static ref FUMOS_CHANNEL_ID: ChannelId = var("FUMOS_CHANNEL_ID")
        .expect("FUMOS_CHANNEL_ID NOT SET")
        .parse()
        .expect("FUMOS_CHANNEL_ID is not a valid Channel ID");
    static ref USERS_IN_BLACKLIST: Vec<UserId> = var("USERSINBLACKLIST")
        .expect("USERSINBLACKLIST must be set")
        .split(',')
        .map(|s| s.parse().expect("Invalid user ID in USERSINBLACKLIST"))
        .collect();
    static ref DISCORD_TOKEN: String = var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::Message { new_message: msg } => {
            if msg.author.bot {
                return Ok(());
            }
            if msg.channel_id == *FUMOS_CHANNEL_ID {
                if msg.attachments.len() == 0 {
                    msg.reply(ctx, format!("Please attach a fumo image to your message"))
                        .await?;
                    return Ok(());
                }
                let reply_msg = CreateMessage::new()
                    .content(format!(
                        "<@{}> Fumo submission succesfully sent to review",
                        msg.author.id.to_string()
                    ))
                    .button(
                        CreateButton::new("approve")
                            .label("Approve Fumo")
                            .style(serenity::ButtonStyle::Primary),
                    )
                    .button(
                        CreateButton::new("reject")
                            .label("Reject Fumo")
                            .style(serenity::ButtonStyle::Danger),
                    )
                    .button(
                        CreateButton::new("add_info")
                            .style(serenity::ButtonStyle::Secondary)
                            .label("Add Info about the submission"),
                    );
                msg.channel_id.send_message(ctx, reply_msg).await?;
                return Ok(());
            }
            if msg.content.to_lowercase() == "ping" && msg.author.id != ctx.cache.current_user().id
            {
                msg.reply(ctx, format!("Pong!!!")).await?;
            }
        }
        serenity::FullEvent::InteractionCreate { interaction } => {
            if interaction.as_message_component().is_some() {
                let component = interaction.as_message_component().unwrap();
                let old_msg = &component.message;
                match component.data.custom_id.as_str() {
                    "approve" => {
                        component
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "Your <@{}> fumo submission has been approved 🎉.",
                                        old_msg.author.id.to_string()
                                    )),
                                ),
                            )
                            .await?;
                        todo!("Handle successful fumo submission.")
                    }
                    "reject" => {
                        component
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "Your <@{}> fumo submission has been denied 😟.",
                                        old_msg.author.id.to_string()
                                    )),
                                ),
                            )
                            .await?;
                    }
                    "add_info" => {
                        todo!("Handle add info response")
                        poise::execute_modal_on_component_interaction::<MoreInfoModal>(ctx, component, None, None)
                    }
                    _ => {}
                }
            };
        }
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![commands::help(), commands::hello()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(")".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![poise::Prefix::Literal("hey chat,")],
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        // Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if (*USERS_IN_BLACKLIST).contains(&ctx.author().id) {
                    return Ok(false);
                }
                if false {
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN").expect("Missing `DISCORD_TOKEN` ");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}
