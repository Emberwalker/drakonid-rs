use serde::Deserialize;

/// An abstract representation of a server variable (SVar).
pub trait SVar<'de> {
    /// The underlying type of this SVar. Typical values include `bool`, `String`, `i64` and `u64`.
    type Target: Send + Sync + Deserialize<'de>;

    /// Returns the SVar's key as seen in the JSON config files.
    fn get_key() -> &'static str;

    /// Returns the SVar's human-readable name, which can be presented to operators over Discord.
    fn get_human() -> &'static str;

    /// Gets the default value for this SVar.
    fn get_default() -> Self::Target;
}

/// One-shot macro for bulk defining SVar types. Accepts a name ident (which names the struct "SVar" + name), key name
/// as in the config file, human-readable version of the key for presenting to users, the type this SVar represents
/// (which must be Deserialize from Serde) and an expression that produces a default value of that type.
macro_rules! define_svars {
    ($(($name:ident, $key:expr, $human:expr, $type:ty, $default:expr)),+$(,)*) => {
        // Build mashup replacement macro
        mashup! {
            $(
                svar_mash["@@" $name] = SVar $name;
            )+
        }

        // Apply the mashup replacement macro to produce "SVar" + name idents when producing types.
        $(
            svar_mash! {
                #[derive(Debug)]
                pub struct "@@" $name();
                impl "@@" $name {
                    // We generate these fields so they're visible to Rustdoc, rather than only seeing the type name.
                    // This also in a crude way works around being unable to interpolate in comments.

                    /// The config key represented by this type.
                    pub const KEY: &'static str = $key;
                    /// The human-readable key represented by this type.
                    pub const HUMAN: &'static str = $human;
                }
                impl<'de> SVar<'de> for "@@" $name {
                    type Target = $type;
                    fn get_key() -> &'static str { "@@" $name::KEY }
                    fn get_human() -> &'static str { "@@" $name::HUMAN }
                    fn get_default() -> $type { $default }
                }
            }
        )+
    };
}

// Add new SVars here.
define_svars!(
    (
        AllowNormalCensus,
        "census_allow_normal_users",
        "~~Allow all users to use `!wow census`~~",
        bool,
        true
    ),
    (
        AllowNormalCondenser,
        "condenser_allow_normal_users",
        "Allow all users to use `!shorten`",
        bool,
        true
    ),
    (
        AllowNormalShowme,
        "showme_allow_normal_users",
        "~~Allow all users to use `!wow showme`~~",
        bool,
        true
    ),
    (
        AllowNormalQuotes,
        "quotes_allow_normal_user",
        "~~Allow all users to use `!quotes`~~",
        bool,
        true
    ),
    (
        RmHistAllowSu,
        "rmhist_allow_su",
        "~~Allow superusers to use !rmhist~~",
        bool,
        true
    ),
    (
        UseSnark,
        "snark_enabled",
        "~~Enable snarky responses~~",
        bool,
        false
    ),
    (
        UseGames,
        "games_enabled",
        "~~Enable games commands such as `!roll`~~",
        bool,
        true
    ),
    (
        RollMin,
        "roll_min",
        "~~Default minimum for `!roll`~~",
        i64,
        1i64
    ),
    (
        RollMax,
        "roll_max",
        "~~Default maximum for `!roll`~~",
        i64,
        100i64
    ),
    (
        DiscAllowSu,
        "disc_allow_su",
        "~~Allow superusers to use disciplinary commands~~",
        bool,
        false
    ),
);
