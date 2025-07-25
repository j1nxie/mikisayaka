use poise::serenity_prelude::*;

use crate::{models::quotes::Quote, Context, Error};

#[tracing::instrument(skip(ctx))]
#[poise::command(
    prefix_command,
    guild_only,
    aliases("quotes"),
    subcommands("add_quote", "list_quotes", "delete_quote", "alias")
)]
pub async fn quote(ctx: Context<'_>, #[rest] title: Option<String>) -> Result<(), Error> {
    if let Some(title) = title {
        let result = sqlx::query!(
            r#"
                SELECT DISTINCT
                    q.id, q.title, q.content
                FROM
                    quotes q
                LEFT JOIN
                    quote_aliases qa ON q.id = qa.quote_id
                WHERE
                    q.title = $1 OR qa.alias = $1;
            "#,
            title,
        )
        .fetch_optional(&ctx.data().db)
        .await
        .inspect_err(|e| {
            tracing::error!(err = ?e, title = %title, "an error occurred when fetching quote");
        })?;

        match result {
            Some(quote) => {
                ctx.send(poise::CreateReply::default().content(quote.content))
                    .await
                    .inspect_err(
                        |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                    )?;
            }
            None => {
                ctx.send(
                    poise::CreateReply::default()
                        .content(format!("quote \"{title}\" does not exist.")),
                )
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;
            }
        }
    }

    Ok(())
}

#[poise::command(prefix_command, rename = "add")]
#[tracing::instrument(skip(ctx))]
pub async fn add_quote(
    ctx: Context<'_>,
    title: String,
    #[rest] content: String,
) -> Result<(), Error> {
    let result = sqlx::query!(
        r#"
            INSERT INTO
                quotes (title, content)
            VALUES
                ($1, $2);
        "#,
        title,
        content
    )
    .execute(&ctx.data().db)
    .await
    .inspect_err(|e| {
        tracing::error!(err = ?e, title = %title, content = %content, "an error occurred when adding quote");
    });

    if let Err(e) = result {
        if e.as_database_error().unwrap().is_unique_violation() {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("quote with title \"{title}\" already exists.")),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
        }

        return Ok(());
    }

    ctx.send(poise::CreateReply::default().content(format!("added quote \"{title}\".")))
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}

#[poise::command(prefix_command, rename = "list")]
#[tracing::instrument(skip_all)]
pub async fn list_quotes(ctx: Context<'_>) -> Result<(), Error> {
    let msg = ctx
        .send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("loading... please watch warmly..."),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    let rows = sqlx::query!(
        r#"
            SELECT
                q.id AS "id!",
                q.title,
                q.content,
                GROUP_CONCAT(qa.alias, ', ') as aliases
            FROM quotes q
            LEFT JOIN quote_aliases qa ON q.id = qa.quote_id
            GROUP BY q.id, q.title, q.content
            ORDER BY q.id;
        "#,
    )
    .fetch_all(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching quotes from database"),
    )?;

    let quotes: Vec<Quote> = rows
        .into_iter()
        .map(|row| {
            let aliases = row
                .aliases
                .map(|s| s.split(", ").map(|s| s.to_string()).collect())
                .unwrap_or_default();

            Quote {
                id: row.id,
                title: row.title,
                content: row.content,
                aliases,
            }
        })
        .collect();

    let mut pages: Vec<String> = vec![];
    let mut current_page: usize = 0;

    for (page, chunk) in quotes.chunks(10).enumerate() {
        let mut quote_list_str = String::new();

        for (idx, quote) in chunk.iter().enumerate() {
            let entry_str = if quote.aliases.is_empty() {
                format!("{}. {}\n", idx + 1 + page * 10, quote.title)
            } else {
                format!(
                    "{}. {} ({})\n",
                    idx + 1 + page * 10,
                    quote.title,
                    quote.aliases.join(", ")
                )
            };

            quote_list_str = quote_list_str + &entry_str;
        }

        pages.push(quote_list_str);
    }

    if pages.is_empty() {
        msg.edit(
            ctx,
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("no quotes found in database!"),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

        return Ok(());
    }

    let ctx_id = ctx.id();
    let author_id = ctx.author().id;
    let first_id = format!("{}first", ctx_id);
    let last_id = format!("{}last", ctx_id);
    let prev_id = format!("{}prev", ctx_id);
    let next_id = format!("{}next", ctx_id);

    msg.edit(
        ctx,
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content("here's your quotes list!")
            .embed(
                CreateEmbed::default()
                    .title("list of quotes")
                    .description(pages[0].clone())
                    .footer(CreateEmbedFooter::new(format!(
                        "page {}/{}",
                        current_page + 1,
                        pages.len(),
                    ))),
            )
            .components(vec![CreateActionRow::Buttons(vec![
                CreateButton::new(&first_id).emoji('⏮').disabled(true),
                CreateButton::new(&prev_id).emoji('◀').disabled(true),
                CreateButton::new(&next_id)
                    .emoji('▶')
                    .disabled(current_page == pages.len() - 1),
                CreateButton::new(&last_id)
                    .emoji('⏭')
                    .disabled(current_page == pages.len() - 1),
            ])]),
    )
    .await
    .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

    while let Some(press) = collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        if press.user.id != author_id {
            press
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("you cannot interact with another user's invoked command!")
                            .ephemeral(true),
                    ),
                )
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when creating response"),
                )?;

            continue;
        }

        if press.data.custom_id == prev_id {
            current_page = current_page.saturating_sub(1);
        } else if press.data.custom_id == next_id {
            current_page += 1;
        } else if press.data.custom_id == first_id {
            current_page = 0;
        } else if press.data.custom_id == last_id {
            current_page = pages.len() - 1;
        } else {
            continue;
        }

        press
            .create_response(
                ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(
                            CreateEmbed::default()
                                .title("list of quotes")
                                .description(pages[current_page].clone())
                                .footer(CreateEmbedFooter::new(format!(
                                    "page {}/{}",
                                    current_page + 1,
                                    pages.len(),
                                ))),
                        )
                        .components(vec![CreateActionRow::Buttons(vec![
                            CreateButton::new(&first_id)
                                .emoji('⏮')
                                .disabled(current_page == 0),
                            CreateButton::new(&prev_id)
                                .emoji('◀')
                                .disabled(current_page == 0),
                            CreateButton::new(&next_id)
                                .emoji('▶')
                                .disabled(current_page == pages.len() - 1),
                            CreateButton::new(&last_id)
                                .emoji('⏭')
                                .disabled(current_page == pages.len() - 1),
                        ])]),
                ),
            )
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, "an error occurred when creating response"),
            )?;
    }

    msg.into_message()
        .await?
        .edit(ctx, EditMessage::default().components(vec![]))
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

    Ok(())
}

#[tracing::instrument(skip(ctx))]
#[poise::command(prefix_command, rename = "delete")]
pub async fn delete_quote(ctx: Context<'_>, #[rest] title: String) -> Result<(), Error> {
    let existing_quote = sqlx::query!(
        r#"
            SELECT
                id AS "id!",
                title,
                content
            FROM quotes
            WHERE title = $1;
        "#,
        title
    )
    .fetch_optional(&ctx.data().db)
    .await
    .inspect_err(|e| {
        tracing::error!(err = ?e, title = %title, "an error occurred when fetching quote");
    })?;

    match existing_quote {
        Some(_) => {
            sqlx::query!(
                r#"
                    DELETE FROM quotes
                    WHERE title = $1;
                "#,
                title
            )
            .execute(&ctx.data().db)
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, title = %title, "an error occurred when deleting quote")
            )?;

            ctx.send(poise::CreateReply::default().content(format!("deleted quote \"{title}\".")))
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default().content(format!("quote \"{title}\" does not exist.")),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
#[poise::command(
    prefix_command,
    subcommand_required,
    subcommands("add_alias", "delete_alias")
)]
pub async fn alias(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[tracing::instrument(skip(ctx))]
#[poise::command(prefix_command, rename = "add")]
pub async fn add_alias(ctx: Context<'_>, quote: String, alias: String) -> Result<(), Error> {
    let existing_quote = sqlx::query!(
        r#"
            SELECT
                id as "id!"
            FROM
                quotes
            WHERE
                title = $1;
        "#,
        quote
    )
    .fetch_optional(&ctx.data().db)
    .await
    .inspect_err(|e| {
        tracing::error!(err = ?e, title = %quote, "an error occurred when fetching quote");
    })?;

    match existing_quote {
        Some(q) => {
            let result = sqlx::query!(
                r#"
                    INSERT INTO
                        quote_aliases (quote_id, alias)
                    VALUES
                        ($1, $2);
                "#,
                q.id,
                alias,
            )
            .execute(&ctx.data().db)
            .await
            .inspect_err(|e| {
                tracing::error!(err = ?e, title = %quote, alias = %alias, "an error occurred when adding alias for quote");
            });

            if let Err(e) = result {
                if e.as_database_error().unwrap().is_unique_violation() {
                    ctx.send(
                        poise::CreateReply::default()
                            .content(format!("alias \"{alias}\" already exists.")),
                    )
                    .await
                    .inspect_err(
                        |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                    )?;
                }

                return Ok(());
            }

            ctx.send(
                poise::CreateReply::default()
                    .content(format!("added alias \"{alias}\" for quote \"{quote}\".")),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default().content(format!("quote \"{quote}\" does not exist.")),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip(ctx))]
#[poise::command(prefix_command, rename = "delete")]
pub async fn delete_alias(ctx: Context<'_>, #[rest] alias: String) -> Result<(), Error> {
    let existing_alias = sqlx::query!(
        r#"
            SELECT
                id AS "id!"
            FROM quote_aliases
            WHERE alias = $1;
        "#,
        alias
    )
    .fetch_optional(&ctx.data().db)
    .await
    .inspect_err(|e| {
        tracing::error!(err = ?e, alias = %alias, "an error occurred when fetching alias");
    })?;

    match existing_alias {
        Some(_) => {
            sqlx::query!(
                r#"
                    DELETE FROM quote_aliases
                    WHERE alias = $1;
                "#,
                alias
            )
            .execute(&ctx.data().db)
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, alias = %alias, "an error occurred when deleting alias")
            )?;

            ctx.send(poise::CreateReply::default().content(format!("deleted alias \"{alias}\".")))
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default().content(format!("alias \"{alias}\" does not exist.")),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
        }
    }

    Ok(())
}
