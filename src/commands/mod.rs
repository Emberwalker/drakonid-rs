use std::process;
use std::sync::Arc;
use serenity::prelude::*;
use serenity::model::id::UserId;
use serenity::framework::StandardFramework;
use serenity::client::bridge::gateway::ShardManager;

use types::ConfigMarker;

static mut SHARD_MANAGER: Option<Arc<Mutex<ShardManager>>> = None;

pub fn attach_framework(client: &mut Client) {
    let conf = Arc::clone(client.data.lock().get::<ConfigMarker>().unwrap());

    // This is technically unsafe, but we know it's always safe.
    // We store this so the stop command can access it - exec's type signature disallows capturing closures.
    unsafe {
        SHARD_MANAGER = Some(Arc::clone(&client.shard_manager));
    }

    client.with_framework(StandardFramework::new()
        .configure(|framework_conf| {
            framework_conf
                .prefix("!")
                .depth(3) // Maximum command segments
                .on_mention(true)
                .owners(conf.get::<Vec<u64>>("discord.owners")
                            .unwrap_or_else(|_| Vec::new())
                            .iter()
                            .map(|it| UserId(*it))
                            .collect())
        })
        .before(|_ctx, msg, cmd_name| {
            debug!("Command execution: '{}' from {} ('{}')", cmd_name, msg.author.id, msg.author.name);
            true
        })
        .bucket("ping", 0, 2, 10)
        .command("ping", |c| c
            .desc("Are you still there?")
            .bucket("ping")
            .exec(|_ctx, msg, _args| {
                let _ = msg.channel_id.say(format!("{} Pong!", msg.author.mention()));
                Ok(())
            })
        )
        .command("stop", |c| c
            .desc("Stops the bot")
            .owners_only(true)
            .exec(|ctx, msg, _| {
                ctx.invisible();
                info!("Shutdown started by {}", msg.author.tag());
                let _ = msg.channel_id.say(format!("{} Shutting down.", msg.author.mention()));
                ctx.quit();

                // Pull the shard manager. We didn't capture it in the closure due to exec's signature.
                unsafe {
                    if let Some(ref manager_lock) = SHARD_MANAGER {
                        manager_lock.lock().shutdown_all();
                    }
                }

                process::exit(0);
            })
        ));
}
