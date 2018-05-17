use std::sync::Arc;
use std::time::Duration;

use chrono::offset::FixedOffset;
use chrono::DateTime;
use reqwest::header::{Headers, UserAgent};
use reqwest::{self, Client, StatusCode};
use serenity::framework::standard::{Args, Command, CommandError, CommandOptions};
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::{Context, Mentionable, Mutex};
use typemap::ShareMap;
use url::Url;

use constants::*;
use types::ConfigMarker;
use utils::{error_embed, usage_error_embed};
use workers::run_on_worker;

thread_local! {
    // Per-thread instance Reqwest's client.
    static REQWEST_CLIENT: Client = {
        let mut headers = Headers::new();
        headers.set(UserAgent::new(USER_AGENT));
        Client::builder()
            .default_headers(headers)
            .timeout(Some(Duration::from_secs(10)))
            .build()
            .expect("Reqwest client init")
    };
}

header!{ (XApiKey, "X-API-Key") => [String] }

#[derive(Serialize, Debug)]
struct ShortenRequest {
    url: String,
    code: Option<String>,
    meta: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ShortenResponse {
    #[serde(with = "url_serde")]
    short_url: Url,
}

#[derive(Serialize, Debug)]
struct DeleteRequest {
    code: String,
}

#[derive(Deserialize, Debug)]
struct DeleteResponse {
    code: String,
    status: String,
}

#[derive(Deserialize, Debug)]
struct MetaResponse {
    #[serde(with = "url_serde")]
    full_url: Url,
    meta: LinkMetadata,
}

#[derive(Deserialize, Debug)]
struct LinkMetadata {
    owner: String,
    time: DateTime<FixedOffset>,
    user_meta: Option<String>,
}

fn handle_response_err(err: reqwest::Error, channel_id: ChannelId, usr_mention: &str) {
    match err.status() {
        Some(code) => {
            warn!(
                "Invalid response but with a HTTP status code? {} -> {:?}",
                code, err
            );
            error_embed(
                &channel_id,
                "A HTTP error occured when communicating with Condenser. Ask your admin for assistance.",
                Some(&usr_mention),
                |e| e
            );
        }
        None => {
            warn!("Error sending Condenser shorten request: {:?}", err);
            error_embed(
                &channel_id,
                "An error occurred when communicating with Condenser. Ask your admin for assistance.",
                Some(&usr_mention),
                |e| e
            );
        }
    }
}

fn handle_response_code(code: StatusCode, channel_id: ChannelId, usr_mention: &str) {
    match code {
        StatusCode::Unauthorized => {
            error_embed(
                &channel_id,
                "The bot's API key is invalid. Ask your admin for assistance.",
                Some(&usr_mention),
                |e| e,
            );
        }
        code => {
            warn!("Unhandled status code: {}", code);
            error_embed(
                &channel_id,
                "An unknown error occurred when communicating with Condenser. Ask your admin for assistance.",
                Some(&usr_mention),
                |e| e
            );
        }
    }
}

fn handle_parse_err(err: reqwest::Error, channel_id: ChannelId, usr_mention: &str) {
    warn!("Error parsing response: {:?}", err);
    error_embed(
        &channel_id,
        "Unable to parse response from server.",
        Some(&usr_mention),
        |e| e,
    );
}

fn url_append(
    base: Url,
    append: &str,
    channel_id: &ChannelId,
    usr_mention: &str,
) -> Result<Url, ()> {
    match base.join(append) {
        Ok(url) => Ok(url),
        Err(err) => {
            warn!("URL construction error: {}", err);
            error_embed(channel_id, "Invalid code.", Some(usr_mention), |e| e);
            Err(())
        }
    }
}

/// Serenity command for shortening URLs with Condenser.
pub struct CondenserShorten {
    opts: Arc<CommandOptions>,
    key: String,
    server: Url,
}

impl CondenserShorten {
    pub fn new(client_data: &Arc<Mutex<ShareMap>>) -> Option<CondenserShorten> {
        // Get configuration out of client data.
        let conf_res = || -> Result<_, ()> {
            let conf = conf!(client_data);
            let key = conf.get_str(CONF_CONDENSER_KEY).map_err(|_| ())?;
            let srv: String = conf.get_str(CONF_CONDENSER_SRV).map_err(|_| ())?;
            Ok((key, Url::parse(&srv).map_err(|_| ())?))
        };

        if let Ok((key, server)) = conf_res() {
            let mut opts = CommandOptions::default();
            opts.desc = Some(format!(
                "Shorten a URL with the Condenser service at {}",
                server
            ));
            opts.usage = Some("[CODE] URL".into());
            opts.example = Some("google https://google.com/".into());
            opts.min_args = Some(1);
            opts.max_args = Some(2);

            Some(CondenserShorten {
                opts: Arc::new(opts),
                key,
                server,
            })
        } else {
            None
        }
    }
}

impl Command for CondenserShorten {
    fn execute(
        &self,
        _ctx: &mut Context,
        msg: &Message,
        mut args: Args,
    ) -> Result<(), CommandError> {
        let argc = args.len();
        if argc < 1 || argc > 2 {
            usage_error_embed(
                "shorten",
                "Wrong number of arguments (must be 1 or 2)",
                Arc::clone(&self.opts),
                msg,
            );
            return Ok(());
        }

        let url: Url;
        let mut code: Option<String> = None;

        if argc == 1 {
            url = match args.single::<Url>() {
                Err(_) => {
                    usage_error_embed(
                        "shorten",
                        "Unable to parse provided URL.",
                        Arc::clone(&self.opts),
                        msg,
                    );
                    return Ok(());
                }
                Ok(url) => url,
            };
        } else {
            url = match args.find::<Url>() {
                Err(_) => {
                    usage_error_embed(
                        "shorten",
                        "Unable to find a valid URL.",
                        Arc::clone(&self.opts),
                        msg,
                    );
                    return Ok(());
                }
                Ok(url) => url,
            };
            code = Some(args.single::<String>().unwrap().to_uppercase()); // We know a string will always be available.
        }

        if url.scheme() != "http" && url.scheme() != "https" {
            usage_error_embed(
                "shorten",
                &format!("Invalid URL scheme: {}", url.scheme()),
                Arc::clone(&self.opts),
                msg,
            );
            return Ok(());
        }

        let srv_name = if let Some(guild) = msg.guild() {
            guild.read().name.clone()
        } else {
            "PM".into()
        };

        let request = ShortenRequest {
            url: url.into_string(),
            code,
            meta: Some(format!(
                "Submitted via Drakonid by {} (via {})",
                msg.author.tag(),
                srv_name
            )),
        };

        // Gather everything the closure will need here.
        let usr_mention = msg.author.mention();
        let channel_id = msg.channel_id;
        let api_key = self.key.clone();
        let mut server_url = self.server.clone();
        server_url.set_path("/api/shorten");

        // Hand off to the worker thread pool.
        run_on_worker(move || {
            //let client = Client::new();
            let response_result = REQWEST_CLIENT.with(|client| {
                client
                    .post(server_url)
                    .header(XApiKey(api_key))
                    .json(&request)
                    .send()
            });

            let mut response = match response_result {
                Ok(res) => res,
                Err(it) => {
                    handle_response_err(it, channel_id, &usr_mention);
                    return;
                }
            };

            let parsed_response: Url = match response.status() {
                StatusCode::Ok => match response.json::<ShortenResponse>() {
                    Ok(res) => res.short_url,
                    Err(err) => {
                        handle_parse_err(err, channel_id, &usr_mention);
                        return;
                    }
                },
                StatusCode::Conflict => {
                    // Handle Conflict here as it only occurs for shorten
                    error_embed(
                        &channel_id,
                        "The provided code already exists.",
                        Some(&usr_mention),
                        |mut e| {
                            if let Some(code) = request.code {
                                e = e.field("Conflicting Code", code, true);
                            }
                            e
                        },
                    );
                    return;
                }
                code => {
                    handle_response_code(code, channel_id, &usr_mention);
                    return;
                }
            };

            let _ = channel_id.send_message(|m| {
                m.content(usr_mention).embed(|e| {
                    e.title("URL Shortened")
                        .colour(*COLOUR_CONDENSER)
                        .field("Short URL", parsed_response.into_string(), false)
                        .field("Original URL", request.url, false)
                })
            });
        });

        Ok(())
    }

    fn options(&self) -> Arc<CommandOptions> {
        Arc::clone(&self.opts)
    }
}

pub struct CondenserMeta {
    opts: Arc<CommandOptions>,
    server: Url,
}

impl CondenserMeta {
    pub fn new(client_data: &Arc<Mutex<ShareMap>>) -> Option<CondenserMeta> {
        // Get configuration out of client data.
        let conf_res = || -> Result<_, ()> {
            let conf = conf!(client_data);
            let srv: String = conf.get_str(CONF_CONDENSER_SRV).map_err(|_| ())?;
            Ok(Url::parse(&srv).map_err(|_| ())?)
        };

        if let Ok(server) = conf_res() {
            let mut opts = CommandOptions::default();
            opts.desc = Some(format!(
                "Fetch metadata for a shortcode on the Condenser service at {}",
                server
            ));
            opts.usage = Some("CODE".into());
            opts.example = Some("google".into());
            opts.min_args = Some(1);
            opts.max_args = Some(1);

            Some(CondenserMeta {
                opts: Arc::new(opts),
                server,
            })
        } else {
            None
        }
    }
}

impl Command for CondenserMeta {
    fn execute(
        &self,
        _ctx: &mut Context,
        msg: &Message,
        mut args: Args,
    ) -> Result<(), CommandError> {
        let code = match args.single::<String>() {
            Ok(code) => code.to_uppercase(),
            Err(_) => {
                usage_error_embed(
                    "condenser meta",
                    "No code specified.",
                    Arc::clone(&self.opts),
                    msg,
                );
                return Ok(());
            }
        };

        // Gather everything the closure will need here.
        let usr_mention = msg.author.mention();
        let channel_id = msg.channel_id;
        let mut server_base = self.server.clone();
        let mut server_url = server_base.clone();
        server_url.set_path("/api/meta/");
        server_url = match url_append(server_url, &code, &channel_id, &usr_mention) {
            Ok(url) => url,
            Err(_) => {
                return Ok(());
            }
        };

        run_on_worker(move || {
            let response_result = REQWEST_CLIENT.with(|client| client.get(server_url).send());

            let mut response = match response_result {
                Ok(res) => res,
                Err(it) => {
                    handle_response_err(it, channel_id, &usr_mention);
                    return;
                }
            };

            let parsed_response: MetaResponse = match response.status() {
                StatusCode::Ok => match response.json::<MetaResponse>() {
                    Ok(res) => res,
                    Err(err) => {
                        handle_parse_err(err, channel_id, &usr_mention);
                        return;
                    }
                },
                StatusCode::NotFound => {
                    error_embed(
                        &channel_id,
                        "Code does not exist.",
                        Some(&usr_mention),
                        |e| e.field("Code", code, false),
                    );
                    return;
                }
                code => {
                    handle_response_code(code, channel_id, &usr_mention);
                    return;
                }
            };

            server_base.set_path(&code);

            let _ = channel_id.send_message(|m| {
                m.content(usr_mention).embed(|mut e| {
                    e = e.title(format!("Metadata for code '{}'", code))
                        .colour(*COLOUR_CONDENSER)
                        .field("Short URL", server_base.into_string(), false)
                        .field("Full URL", parsed_response.full_url, false)
                        .field("Owner", parsed_response.meta.owner, true)
                        .field(
                            "Created At",
                            parsed_response
                                .meta
                                .time
                                .format("%d/%m/%Y at %H:%M:%S (%Z)"),
                            true,
                        );

                    if let Some(meta) = parsed_response.meta.user_meta {
                        if meta != "" {
                            e = e.field("User Metadata", meta, true);
                        }
                    }

                    e
                })
            });
        });

        Ok(())
    }

    fn options(&self) -> Arc<CommandOptions> {
        Arc::clone(&self.opts)
    }
}
