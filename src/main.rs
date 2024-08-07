#![warn(clippy::str_to_string)]

mod commands;

use ::serenity::all::{
    ChannelId, ComponentInteraction, CreateButton, CreateInteractionResponseMessage, CreateMessage,
    Message, UserId,
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
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{env::var, sync::Arc, time::Duration};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    fumos_collection: MongoCollection<FumoDoc>,
    submissions_collection: MongoCollection<SubmissionDoc>,
    fumo_api_endpoint: String,
    web_client: reqwest::Client,
    curators: Vec<UserId>,
}

#[derive(Debug, poise::Modal)]
struct MoreInfoModal {
    credit: Option<String>,
    source: Option<String>,
    caption: Option<String>,
    featured: Option<String>, // plushie featured in the image
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubmissionDoc {
    _id: String,
    image_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    featured: Option<String>,
    approved: bool,
    discarted: bool,
    discord_submitter_id: String,
    time_of_submission: i64,
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
                        "<@{}> Fumo submission succesfully sent to review \n -# Only the first media attachment is going to be considered",
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

                let submission = SubmissionDoc {
                    _id: msg.id.to_string(),
                    image_url: msg.attachments[0].url.clone(),
                    caption: None,
                    source: None,
                    credit: None,
                    featured: None,
                    approved: false,
                    discarted: false,
                    discord_submitter_id: msg.author.id.to_string(),
                    time_of_submission: msg.timestamp.timestamp(),
                };

                data.submissions_collection
                    .insert_one(submission)
                    .await
                    .expect("Failed to insert submission into database");
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
                let mut component_copy: ComponentInteraction = component.clone();
                let mut old_msg = component_copy.message;
                match component.data.custom_id.as_str() {
                    "approve" => {
                        if !data.curators.contains(&component.user.id) {
                            component
                                .create_response(
                                    ctx,
                                    serenity::CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new().content(
                                            "You are not a curator, you can't approve fumos",
                                        ),
                                    ),
                                )
                                .await?;
                            return Ok(());
                        }
                        let referenced = &old_msg
                            .message_reference
                            .as_ref()
                            .expect("No referenced message in fumo submission reply")
                            .message_id
                            .unwrap();

                        let channel = &component.channel_id;
                        let referenced = channel.message(&ctx.http, referenced).await?;
                        let submission_creator_id = referenced.author.id.to_string();
                        println!("{}", submission_creator_id);
                        component
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "<@{}> Your fumo submission has been approved 🎉. You can check it out with id `{}`",
                                        submission_creator_id,
                                        referenced.id.to_string()
                                    )),
                                ),
                            )
                            .await?;

                        #[rustfmt::skip]
                       // add_fumo_to_db(&data.fumos_collection, fumo_to_create).await.expect("Failed to add fumo to db");
                        data.submissions_collection
                            .update_one(
                                doc! {
                                    "_id": old_msg
                                        .message_reference
                                        .as_ref()
                                        .expect("No referenced message in fumo submission reply")
                                        .message_id.unwrap().to_string()
                                },
                                doc! {
                                    "$set": {
                                        "approved": true
                                    }
                                },
                            )
                            .await
                            .expect("Error while giving fumo the approved flag");
                    }
                    "reject" => {
                        if !data.curators.contains(&component.user.id) {
                            component
                                .create_response(
                                    ctx,
                                    serenity::CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new().content(
                                            "You are not a curator, you can't approve fumos",
                                        ),
                                    ),
                                )
                                .await?;
                            return Ok(());
                        }
                        let referenced = &old_msg
                            .message_reference
                            .as_ref()
                            .expect("No referenced message in fumo submission reply")
                            .message_id
                            .unwrap();
                        let channel = &component.channel_id;
                        let referenced = channel.message(&ctx.http, referenced).await?;
                        let submission_creator_id = referenced.author.id.to_string();
                        data.fumos_collection
                            .update_one(
                                doc! {
                                    "_id": referenced.id.to_string()
                                },
                                doc! {
                                    "$set": {
                                        "discarted": true
                                    }
                                },
                            )
                            .await
                            .expect("Failed to reject fumo in db (add discarted flag)");
                        component
                            .create_response(
                                ctx,
                                serenity::CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "<@{}> Your fumo submission has been denied 😟.",
                                        referenced.author.id.to_string()
                                    )),
                                ),
                            )
                            .await?;
                    }
                    "add_info" => {
                        println!("{:?}", old_msg);

                        let modal_response = poise::execute_modal_on_component_interaction::<
                            MoreInfoModal,
                        >(
                            Box::new(ctx.clone()), component.clone(), None, None
                        )
                        .await?
                        .ok_or("Couldnt parse modal successfully");

                        #[rustfmt::skip]
                        let MoreInfoModal {caption, credit, source, featured} = &modal_response.unwrap();
                        data.submissions_collection.update_one(doc!{
                            "_id": old_msg.message_reference.as_ref().expect("No referenced message in fumo submission reply").message_id.unwrap().to_string()
                        }, doc!{
                            "$set": {
                                "caption": caption,
                                "credit": credit,
                                "source": source,
                                "featured": featured
                            }
                        },).await.expect("Failed to update submission with more info");

                        old_msg
                            .edit(
                                ctx,
                                //edit message to add a embed
                                serenity::EditMessage::new().add_embed(
                                    serenity::CreateEmbed::new()
                                        .description("More info succesfully added"),
                                ),
                            )
                            .await
                            .expect("Failed to edit message to add embed");
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
    let fumo_api_endpoint = "http://localhost:6969".to_string();

    let web_client = reqwest::Client::new();

    let db = mongo.database("fumo-api");
    let fumos_collection = db.collection("fumos");
    let submissions_collection = db.collection("submissions");

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![
            commands::help(),
            commands::hello(),
            commands::fumo(),
            commands::random(),
        ],
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
                Ok(Data {
                    fumos_collection,
                    submissions_collection,
                    web_client,
                    fumo_api_endpoint,
                    curators: vec![UserId::from(688476559019212805)],
                })
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

fn upload_to_nosesisaid_cdn(image: &String) -> Result<String, Box<dyn std::error::Error>> {
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
