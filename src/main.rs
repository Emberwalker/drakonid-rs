extern crate chrono;
#[macro_use]
extern crate clap;
extern crate fern;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate regex;

extern crate drakonid;

use clap::{App, Arg};
use regex::Regex;
use std::{cmp, thread};
use std::hash::{Hash, Hasher};

static CONF_LOC_ENV: &'static str = "DRAKONID_CONF";
static DEFAULT_CONF_LOC: &'static str = "./config";

// From https://docs.rs/console/0.6.1/src/console/utils.rs.html#12
lazy_static! {
    static ref STRIP_ANSI_RE: Regex = Regex::new(
        r"[\x1b\x9b][\[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-PRZcf-nqry=><]"
    ).unwrap();
}

fn main() {
    let matches = App::new("Drakonid")
        .version(crate_version!())
        .author("Robert T. <arkan@drakon.io>")
        .about("Discord bot for stuff and things.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Provide a custom configuration file.")
                .env(CONF_LOC_ENV)
                .default_value(DEFAULT_CONF_LOC)
                .global(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets verbosity. May be specified up to 3 times.")
                .global(true),
        )
        .arg(
            Arg::with_name("is_wrapped")
                .long("is_wrapped")
                .help("Pass this flag to tell Drakonid that it has a update-capable wrapper, enabling `!update`.")
                .global(true)
        )
        .get_matches();

    let conf_loc = matches.value_of("config").unwrap_or(DEFAULT_CONF_LOC);
    let log_lvl = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 | _ => log::LevelFilter::Trace,
    };

    if let Err(err) = setup_logger(log_lvl) {
        panic!("Error setting up logger: {}", err);
    }

    info!(target: "main", "Logger configured; using log level {}", log_lvl);

    drakonid::run(conf_loc, matches.occurrences_of("is_wrapped") != 0);
}

// We use this fake hasher to get raw thread IDs.
struct DummyHasher(pub u64);

impl Hasher for DummyHasher {
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn write(&mut self, _: &[u8]) {} // NOOP

    fn finish(&self) -> u64 {
        self.0
    }
}

fn get_thread() -> String {
    let id = {
        let mut h = DummyHasher(0);
        thread::current().id().hash(&mut h);
        format!("{}", h.finish())
    };

    let name: String = thread::current().name().unwrap_or_else(|| "<unnamed>").into();

    format!("{}/{}", id, name)
}

fn setup_logger(lvl: log::LevelFilter) -> Result<(), fern::InitError> {
    let noisy_crate_lvl = cmp::min(log::LevelFilter::Warn, lvl); // For VERY noisy crates
    let verbose_crate_lvl = cmp::min(log::LevelFilter::Info, lvl); // For somewhat noisy crates
    fern::Dispatch::new()
        .format(|out, message, record| {
            if record.target().starts_with("drakonid") || record.target().starts_with("main") {
                out.finish(format_args!(
                    "[{}][{}][{}][{}][{}:{}] {}",
                    chrono::Utc::now().format("%Y/%m/%d %H:%M:%S%.3f%z"),
                    get_thread(),
                    record.level(),
                    record.target(),
                    record.file().unwrap_or("<unknown>"),
                    record
                        .line()
                        .map(|it| it.to_string())
                        .unwrap_or_else(|| "???".to_string()),
                    message
                ))
            } else {
                // We drop the file info for dependencies, since their file paths are long and absolute.
                // Also strip any ANSI sequences, in case anything odd gets logged.
                out.finish(format_args!(
                    "[{}][{}][{}][{}][<elided>:???] {}",
                    chrono::Utc::now().format("%Y/%m/%d %H:%M:%S%.3f%z"),
                    get_thread(),
                    record.level(),
                    record.target(),
                    STRIP_ANSI_RE.replace_all(&format!("{}", message), "")
                ))
            }
        })
        .level(lvl)
        // Set log level to WARN or the assigned level, whichever is least verbose, for noisy crates.
        .level_for("hyper", noisy_crate_lvl)
        .level_for("tokio_core", noisy_crate_lvl)
        .level_for("tokio_reactor", noisy_crate_lvl)
        .level_for("evzht9h3nznqzwl", noisy_crate_lvl) // rust-websocket Serenity fork
        .level_for("serenity", verbose_crate_lvl)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
