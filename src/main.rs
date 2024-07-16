#![warn(clippy::str_to_string)]

mod commands;

use ::serenity::all::{
    ChannelId, CreateButton, CreateInteractionResponseMessage, CreateMessage, UserId,
};
use commands::Fumo;
use dotenv::dotenv;
use env_logger;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Bson, Document},
    Client as MongoClient, Collection as MongoCollection,
};
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use std::{env::var, sync::Arc, time::Duration};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    fumos_collection: MongoCollection<FumoDoc>,
}

#[derive(Debug, poise::Modal)]
struct MoreInfoModal {
    credit: Option<String>,
    source: Option<String>,
    caption: Option<String>,
    featured: Option<String>, // plushie featured in the image
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FumoDoc {
    _id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
    image_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    featured: Option<String>,
}

lazy_static! {
    static ref FUMOS_CHANNEL_ID: ChannelId = var("FUMOS_CHANNEL_ID")
        .expect("FUMOS_CHANNEL_ID NOT SET")
        .parse()
        .expect("FUMOS_CHANNEL_ID is not a valid Channel ID");
    static ref USERS_IN_BLACKLIST: Vec<UserId> = var("USERS_IN_BLACKLIST")
        .expect("USERSINBLACKLIST must be set")
        .split(',')
        .map(|s| s.parse().expect("Invalid user ID in USERSINBLACKLIST"))
        .collect();
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
                    )
                    .reference_message(msg);
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
                        let submission_creator_id = old_msg
                            .referenced_message
                            .as_ref()
                            .unwrap()
                            .author
                            .id
                            .to_string();
                        println!("{}", submission_creator_id);
                        component
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "<@{}> Your fumo submission has been approved 🎉. You can check it out with id `{}`",
                                        old_msg
                                            .referenced_message
                                            .as_ref()
                                            .expect(
                                                "No referenced message in fumo submission reply"
                                            )
                                            .author
                                            .id
                                            .to_string(),
                                        old_msg
                                            .referenced_message
                                            .as_ref()
                                            .expect(
                                                "No referenced message in fumo submission reply"
                                            )
                                            .id
                                            .to_string()
                                    )),
                                ),
                            )
                            .await?;

                        let fumo_to_create = Fumo {
                            _id: old_msg
                                .referenced_message
                                .as_ref()
                                .expect("No referenced message in fumo submission reply")
                                .id
                                .to_string(),
                            caption: None,
                            image: old_msg
                                .referenced_message
                                .as_ref()
                                .expect("No referenced message in fumo submission reply")
                                .attachments
                                .first()
                                .expect("No attachments in fumo submission reply")
                                .url
                                .clone(),
                            source: None,
                            credit: None,
                            featured: None,
                        };
                        #[rustfmt::skip]
                        add_fumo_to_db(&data.fumos_collection, fumo_to_create).await.expect("Failed to add fumo to db");
                    }
                    "reject" => {
                        component
                            .create_response(
                                &ctx,
                                serenity::CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "<@{}> Your fumo submission has been denied 😟.",
                                        old_msg
                                            .referenced_message
                                            .as_ref()
                                            .expect(
                                                "No referenced message in fumo submission reply"
                                            )
                                            .author
                                            .id
                                            .to_string()
                                    )),
                                ),
                            )
                            .await?;
                    }
                    "add_info" => {
                        let modal_response = poise::execute_modal_on_component_interaction::<
                            MoreInfoModal,
                        >(
                            Box::new(ctx.clone()), component.clone(), None, None
                        )
                        .await?
                        .ok_or("Couldnt parse modal successfully");

                        #[rustfmt::skip]
                        let MoreInfoModal {caption, credit, source, featured} = modal_response.unwrap();
                        todo!("Handle modal response")
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

    let MONGO_URI = std::env::var("MONGO_URI").expect("Expected a mongo uri in the environment");
    let mongo = mongodb::Client::with_uri_str(MONGO_URI).await.unwrap();

    let db = mongo.database("fumo");
    let fumos_collection = db.collection("fumos");

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
                Ok(Data { fumos_collection })
            })
        })
        .options(options)
        .build();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap()
}

fn upload_to_nosesisaid_cdn(image: &str) -> Result<String, Box<dyn std::error::Error>> {
    return Ok(String::from("https://cdn.nosesisaid.com/1234.png"));
    todo!("Upload image to nosesisaid cdn (r2 instance)");
}
async fn add_fumo_to_db(
    fumos_collection: &MongoCollection<FumoDoc>,
    fumo: Fumo,
) -> Result<(), Box<dyn std::error::Error>> {
    let cdn_url = upload_to_nosesisaid_cdn(&fumo.image).expect("Failed to upload image to cdn");

    let fumo = FumoDoc {
        _id: fumo._id,
        image_url: cdn_url,
        caption: fumo.caption,
        credit: fumo.credit,
        source: fumo.source,
        featured: fumo.featured,
    };

    fumos_collection.insert_one(fumo).await?;
    Ok(())
}
