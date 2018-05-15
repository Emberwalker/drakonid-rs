#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![allow(unknown_lints)]
#![warn(clippy)]

extern crate config;
extern crate chrono;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate parking_lot;
extern crate rand;
extern crate regex;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serenity;
extern crate threadpool;
extern crate typemap;
extern crate url;

use std::{time, thread};
use std::sync::Arc;

use serenity::prelude::*;
use serenity::Client;
use serenity::model::gateway::Ready;

pub mod workers;
pub mod types;
pub mod constants;
pub mod commands;

const RESTART_SECONDS: u64 = 30;

// TODO: Replace this with the live event handler
struct Handler;
impl EventHandler for Handler {
    fn ready(&self, ctx: Context, _: Ready) {
        ctx.reset_presence();
    }
}

pub fn run(conf_loc: &str, is_wrapped: bool) {
    let mut conf = config::Config::default();
    conf
        .set_default(constants::CONF_IS_WRAPPED, false).unwrap()
        .merge(config::File::with_name(conf_loc.trim_right_matches(".toml").trim_right_matches(".json")).required(true))
        .expect("unable to load configuration")
        .merge(config::Environment::with_prefix("drakonid")).unwrap();
    
    if is_wrapped {
        conf.set(constants::CONF_IS_WRAPPED, true).unwrap();
    }

    let token = conf.get_str(constants::CONF_DISCORD_TOKEN).expect("No token specified in configuration.");
    let mut client = Client::new(&token, Handler).expect("Serenity client init failed.");
    
    // Attach config to Serenity's shared data (which is exposed in Context structs later)
    debug!("Attaching configuration to Client/Context data.");
    {
        let mut lock = client.data.lock();
        lock.insert::<types::ConfigMarker>(Arc::new(conf));
    }

    // Attach Standard Framework
    debug!("Attaching framework to Serenity client.");
    commands::attach_framework(&mut client);

    // Loop and restart automatically on failures.
    info!("Starting Serenity.");
    while let Err(e) = client.start() {
        error!("Serenity client terminated abnormally: {}", e);
        debug!("Additional information: {:?}", e);
        info!("Attempting restart in {} seconds", RESTART_SECONDS);
        thread::sleep(time::Duration::from_secs(RESTART_SECONDS));
    }
}
