use serenity::utils::Colour;

// Config keys
pub const CONF_IS_WRAPPED: &str = "env.wrapped";

pub const CONF_DISCORD_APPID: &str = "discord.appid";
pub const CONF_DISCORD_TOKEN: &str = "discord.token";
pub const CONF_DISCORD_OWNERS: &str = "discord.owners";

pub const CONF_BNET_TOKEN: &str = "battlenet.token";

pub const CONF_CONDENSER_SRV: &str = "condenser.server";
pub const CONF_CONDENSER_KEY: &str = "condenser.key";

// Metadata
pub const USER_AGENT: &str = concat!("drakonid-rs/", env!("CARGO_PKG_VERSION"));

// Colours
lazy_static! {
    // Global colours
    pub static ref COLOUR_PRIMARY: Colour = Colour::orange();
    pub static ref COLOUR_ERROR: Colour = Colour::red();

    // Subsystem specific colours
    pub static ref COLOUR_CONDENSER: Colour = Colour::blue();
}