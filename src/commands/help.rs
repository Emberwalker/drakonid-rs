/// This file is mostly derived from Serenity's with_embed implementation, but with nitpicky formatting changes.

use std::collections::HashMap;
use std::fmt::Write;
use std::hash::BuildHasher;
use std::sync::Arc;

use serenity::prelude::*;
use serenity::framework::standard::{
    has_correct_roles,
    has_correct_permissions,
    Args,
    Command,
    CommandError,
    CommandGroup,
    CommandOrAlias,
    HelpBehaviour,
    HelpFunction,
    HelpOptions,
};
use serenity::framework::standard::help_commands::has_all_requirements;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::utils::Colour;

// This makes sure we're always satisfying HelpFunction.
#[allow(non_upper_case_globals)]
pub const drakonid_help: HelpFunction = with_embeds;

//
// Stuff past this point is just modified versions of the equivalent in Serenity.
//

fn error_embed(channel_id: &ChannelId, input: &str, colour: Colour) {
    let _ = channel_id.send_message(|m| {
        // arkan: Add title to embed to prevent the weird little space at the top of the embed.
        m.embed(|e| e.colour(colour).description(input).title("Error"))
    });
}

fn remove_aliases(cmds: &HashMap<String, CommandOrAlias>) -> HashMap<&String, &Arc<Command>> {
    let mut result = HashMap::new();

    for (n, v) in cmds {
        if let CommandOrAlias::Command(ref cmd) = *v {
            result.insert(n, cmd);
        }
    }

    result
}

fn with_embeds<H: BuildHasher>(
    _: &mut Context,
    msg: &Message,
    help_options: &HelpOptions,
    groups: HashMap<String, Arc<CommandGroup>, H>,
    args: &Args
) -> Result<(), CommandError> {
    if !args.is_empty() {
        let name = args.full();

        for (group_name, group) in groups {
            let mut found: Option<(&String, &Arc<Command>)> = None;

            for (command_name, command) in &group.commands {
                let with_prefix = if let Some(ref prefix) = group.prefix {
                    format!("{} {}", prefix, command_name)
                } else {
                    command_name.to_string()
                };

                if name == with_prefix || name == *command_name {
                    match *command {
                        CommandOrAlias::Command(ref cmd) => {
                            if has_all_requirements(&cmd.options(), msg) {
                                found = Some((command_name, cmd));
                            } else {
                                break;
                            }
                        },
                        CommandOrAlias::Alias(ref name) => {
                            let actual_command = &group.commands[name];

                            match *actual_command {
                                CommandOrAlias::Command(ref cmd) => {
                                    if has_all_requirements(&cmd.options(), msg) {
                                        found = Some((name, cmd));
                                    } else {
                                        break;
                                    }
                                },

                                CommandOrAlias::Alias(ref name) => {
                                    let _ = msg.channel_id.say(help_options.suggestion_text.replace("{}", name));
                                    return Ok(());
                                },
                            }
                        },
                    }
                }
            }

            if let Some((command_name, command)) = found {
                let command = command.options();
                if !command.help_available {
                    error_embed(&msg.channel_id, &help_options.no_help_available_text, help_options.embed_error_colour);

                    return Ok(());
                }

                let _ = msg.channel_id.send_message(|m| {
                    m.embed(|e| {
                        let mut embed = e.colour(help_options.embed_success_colour).title(command_name);

                        if let Some(ref desc) = command.desc {
                            embed = embed.description(desc);
                        }

                        if let Some(ref usage) = command.usage {
                            let value = format!("`{} {}`", command_name, usage);

                            embed = embed.field(&help_options.usage_label, value, true);
                        }

                        if let Some(ref example) = command.example {
                            let value = format!("`{} {}`", command_name, example);

                            embed = embed.field(&help_options.usage_sample_label, value, true);
                        }

                        if group_name != "Ungrouped" {
                            embed = embed.field(&help_options.grouped_label, group_name, true);
                        }

                        if !command.aliases.is_empty() {
                            let aliases = command.aliases.join(", ");

                            embed = embed.field(&help_options.aliases_label, aliases, true);
                        }

                        let available = if command.dm_only {
                            &help_options.dm_only_text
                        } else if command.guild_only {
                            &help_options.guild_only_text
                        } else {
                            &help_options.dm_and_guild_text
                        };

                        embed = embed.field(&help_options.available_text, available, true);

                        embed
                    })
                });

                return Ok(());
            }
        }

        let error_msg = help_options.command_not_found_text.replace("{}", name);
        error_embed(&msg.channel_id, &error_msg, help_options.embed_error_colour);

        return Ok(());
    }

    let _ = msg.channel_id.send_message(|m| {
        m.embed(|mut e| {
            // arkan: Add a title. Not doing this leaves a weird gap at the top of the embed.
            e = e.title("Command List");

            if let Some(ref striked_command_text) = help_options.striked_commands_tip {
                e = e.colour(help_options.embed_success_colour).description(
                    format!("{}\n{}", &help_options.individual_command_tip, striked_command_text),
                );
            } else {
                e = e.colour(help_options.embed_success_colour).description(
                    &help_options.individual_command_tip,
                );
            }

            let mut group_names = groups.keys().collect::<Vec<_>>();
            group_names.sort();

            for group_name in group_names {
                let group = &groups[group_name];
                let mut desc = String::new();

                if let Some(ref x) = group.prefix {
                    let _ = writeln!(desc, "{}: `{}`", &help_options.group_prefix, x);
                }

                let mut has_commands = false;

                let commands = remove_aliases(&group.commands);
                // arkan: Filter out commands with help_available disabled (I want that to *hide* commands)
                let mut command_names = commands
                    .iter()
                    .filter(|(ref _k, cmd)| cmd.options().help_available)
                    .map(move |(k, _cmd)| k)
                    .collect::<Vec<_>>();
                command_names.sort();

                for name in command_names {
                    let cmd = &commands[name];
                    let cmd = cmd.options();

                    if !cmd.dm_only && !cmd.guild_only || cmd.dm_only && msg.is_private() || cmd.guild_only && !msg.is_private() {

                        if cmd.help_available && has_correct_permissions(&cmd, msg) {

                            if let Some(guild) = msg.guild() {
                                let guild = guild.read();

                                if let Some(member) = guild.members.get(&msg.author.id) {

                                    if has_correct_roles(&cmd, &guild, &member) {
                                        let _ = writeln!(desc, "`{}`", name);
                                        has_commands = true;
                                    } else {
                                        match help_options.lacking_role {
                                            HelpBehaviour::Strike => {
                                                let name = format!("~~`{}`~~", &name);
                                                let _ = writeln!(desc, "{}", name);
                                                has_commands = true;
                                            },
                                                HelpBehaviour::Nothing => {
                                                let _ = writeln!(desc, "`{}`", name);
                                                has_commands = true;
                                            },
                                                HelpBehaviour::Hide => {
                                                continue;
                                            },
                                        }
                                    }
                                }
                            } else {
                                let _ = writeln!(desc, "`{}`", name);
                                has_commands = true;
                            }
                        } else {
                            match help_options.lacking_permissions {
                                HelpBehaviour::Strike => {
                                    let name = format!("~~`{}`~~", &name);
                                    let _ = writeln!(desc, "{}", name);
                                    has_commands = true;
                                },
                                HelpBehaviour::Nothing => {
                                    let _ = writeln!(desc, "`{}`", name);
                                    has_commands = true;
                                },
                                HelpBehaviour::Hide => {
                                    continue;
                                },
                            }
                        }
                    } else {
                        match help_options.wrong_channel {
                            HelpBehaviour::Strike => {
                                let name = format!("~~`{}`~~", &name);
                                let _ = writeln!(desc, "{}", name);
                                has_commands = true;
                            },
                            HelpBehaviour::Nothing => {
                                let _ = writeln!(desc, "`{}`", name);
                                has_commands = true;
                            },
                            HelpBehaviour::Hide => {
                                continue;
                            },
                        }
                    }
                }

                if has_commands {
                    e = e.field(&group_name[..], &desc[..], true);
                }
            }
            e
        })
    });

    Ok(())
}