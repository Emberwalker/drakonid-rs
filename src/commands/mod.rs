use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::standard::StandardFramework;
use serenity::model::id::UserId;
use serenity::prelude::*;
use serenity::utils::Colour;
use std::process;
use std::sync::Arc;

mod condenser;
mod help;
mod unimplemented;

use self::unimplemented::UnimplementedCommand;
use constants;
use types::ConfigMarker;

static mut SHARD_MANAGER: Option<Arc<Mutex<ShardManager>>> = None;

fn shutdown_bot() {
    // Pull the shard manager. We didn't capture it in the closure due to exec's signature.
    // We cannot just use Context::quit, since that doesn't force all of Serenity to stop. This does.
    unsafe {
        if let Some(ref manager_lock) = SHARD_MANAGER {
            manager_lock.lock().shutdown_all();
        }
    }
}

pub fn attach_framework(client: &mut Client) {
    let cdata = Arc::clone(&client.data);
    let conf = Arc::clone(cdata.lock().get::<ConfigMarker>().unwrap());

    // This is technically unsafe, but we know it's always safe.
    // We store this so the stop command can access it - exec's type signature disallows capturing closures.
    unsafe {
        SHARD_MANAGER = Some(Arc::clone(&client.shard_manager));
    }

    client.with_framework(
        StandardFramework::new()
        .configure(|framework_conf| { framework_conf
            .prefix("!")
            .depth(3) // Maximum command segments
            .on_mention(true)
            .owners(conf.get::<Vec<u64>>(constants::CONF_DISCORD_OWNERS)
                        .unwrap_or_else(|_| Vec::new())
                        .iter()
                        .map(|it| UserId(*it))
                        .collect())
        })
        .customised_help(help::drakonid_help, |help| help
            .individual_command_tip("For help on a specific command, run `!help` followed by the command name.")
            .striked_commands_tip(
                Some("Striked out commands are not available to you here, but may be available elsewhere.".into())
            )
            // Tweak these two to compensate for the changes in our modified help function.
            .no_help_available_text(":warning: No help available for that command.")
            .command_not_found_text(":skull_crossbones: Command `{}` does not exist.")
            .embed_success_colour(Colour::orange())
        )
        // Command logger
        .before(|_ctx, msg, cmd_name| {
            debug!("Command execution: '{}' from {} ('{}')", cmd_name, msg.author.id, msg.author.name);
            true // We're not a check, so always approve messages.
        })

        // Add buckets below here
        .bucket("ping", 0, 2, 10)

        // Add commands/groups below here
        .group("Actions", |mut group| {
            if let Some(shorten) = condenser::CondenserShorten::new(&cdata) {
                group = group.cmd("shorten", shorten);
            }
            group
        })
        .group("Announcements", |group| group
            .prefix("ann")
            // TODO: Attach anouncements commands here.
            .cmd("join set", UnimplementedCommand::new())
            .cmd("join del", UnimplementedCommand::new())
            .cmd("stream set", UnimplementedCommand::new())
            .cmd("stream del", UnimplementedCommand::new())
        )
        .group("Condenser", |mut group| {
            group = group.prefix("condenser");
            if let Some(meta) = condenser::CondenserMeta::new(&cdata) {
                group = group.cmd("meta", meta);
            }
            if let Some(rm) = condenser::CondenserDelete::new(&cdata) {
                group = group.cmd("delete", rm);
            }
            group
        })
        .group("Permissions", |group| group
            .prefix("perm")
            // TODO: Attach permission management commands here.
            .cmd("set", UnimplementedCommand::new())
        )
        .group("World of Warcraft", |group| group
            .prefix("wow")
            // TODO: Attach Battle.net commands here.
            .cmd("showme", UnimplementedCommand::new())
            .cmd("census", UnimplementedCommand::new())
        )
        .group("Utilities", |mut group| { // Basic utilities. Not worth splitting out into command modules alone.
            group = group
                .command("ping", |c| c
                    .desc("Are you still there?")
                    .bucket("ping")
                    .exec(|_ctx, msg, _args| {
                        let _ = msg.reply("Pong!");
                        Ok(())
                    })
                )
                .command("stop", |c| c
                    .desc("Stops the bot")
                    .owners_only(true)
                    .known_as("stahp")
                    .exec(|ctx, msg, _| {
                        // Set invisible first. This prevents the bot showing as "Online" for a while after terminating.
                        // We reset this in the Ready event, when the bot boots up again.
                        ctx.invisible();
                        warn!("Shutdown started by {}", msg.author.tag());
                        let _ = msg.reply("Shutting down.");

                        shutdown_bot();

                        Ok(())
                    })
                );
            
            // Only enable `!update` if in a wrapper.
            let allow_update = cdata.lock()
                .get::<ConfigMarker>()
                .unwrap()
                .get_bool(constants::CONF_IS_WRAPPED)
                .unwrap_or_else(|_| false);
            
            if allow_update {
                group = group.command("update", |c| c
                    .desc("Triggers a bot update. Only works if the bot is run from a wrapper script and not inside a \
                          Docker container.")
                    .owners_only(true)
                    .exec(|ctx, msg, _| {
                        // See `!stop` for why we do this.
                        ctx.invisible();
                        warn!("Going down for update...");
                        let _ = msg.reply("Shutting down for update.");
                        shutdown_bot();
                        process::exit(-100);
                    })
                )
            }

            group
        }),
    );
}
