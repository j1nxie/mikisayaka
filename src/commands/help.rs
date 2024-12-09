use crate::{Context, Error};

/// print the list of commands and their usage
#[poise::command(slash_command)]
#[tracing::instrument(skip_all)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "specific command to show help about"] command: Option<String>,
) -> Result<(), Error> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "Type `s>help command` for more info on a command.",
        ..Default::default()
    };

    poise::builtins::help(ctx, command.as_deref(), config)
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}
