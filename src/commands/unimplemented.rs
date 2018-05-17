use serenity::framework::standard::{Args, Command, CommandError, CommandOptions};
use serenity::model::channel::Message;
use serenity::prelude::*;
use serenity::utils::Colour;
use std::sync::Arc;

pub struct UnimplementedCommand {
    opts: Arc<CommandOptions>,
}

impl UnimplementedCommand {
    pub fn new() -> UnimplementedCommand {
        let mut opts = CommandOptions::default();
        opts.desc = Some("An unimplemented command.".into());

        UnimplementedCommand {
            opts: Arc::new(opts),
        }
    }
}

impl Command for UnimplementedCommand {
    fn execute(&self, _: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
        let _ = msg.channel_id.send_message(|m| {
            m.embed(|e| {
                e.title("Unimplemented")
                    .description(":construction: This command isn't implemented yet.")
                    .colour(Colour::gold())
            })
        });
        Ok(())
    }

    fn options(&self) -> Arc<CommandOptions> {
        Arc::clone(&self.opts)
    }
}
