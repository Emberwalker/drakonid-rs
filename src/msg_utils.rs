use std::sync::Arc;

use serenity::builder::CreateEmbed;
use serenity::framework::standard::CommandOptions;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;

use constants::COLOUR_ERROR;

pub fn usage_error_embed(cmd_name: &str, err_text: &str, opts: Arc<CommandOptions>, msg: &Message) {
    let _ = msg.channel_id.send_message(|m| {
        m.embed(|mut e| {
            e = e.title("Error").description(err_text).colour(*COLOUR_ERROR);

            if let Some(ref usage) = opts.usage {
                e = e.field("Usage", format!("`!{} {}`", cmd_name, usage), false);
            }
            if let Some(ref example) = opts.example {
                e = e.field("Example", format!("`!{} {}`", cmd_name, example), false);
            }

            e
        })
    });
}

pub fn error_embed<T: FnOnce(CreateEmbed) -> (CreateEmbed)>(
    channel_id: ChannelId,
    err_text: &str,
    mention_text: Option<&str>,
    embed_thunk: T,
) {
    let _ = channel_id.send_message(|mut m| {
        if let Some(text) = mention_text {
            m = m.content(text);
        }

        m.embed(|e| embed_thunk(e.title("Error").description(err_text).colour(*COLOUR_ERROR)))
    });
}
