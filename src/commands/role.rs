use std::collections::HashSet;

use poise::serenity_prelude::*;

use crate::models::roles::Role;
use crate::{Context, Error};

/// commands related to self-assignable roles.
#[tracing::instrument(skip_all)]
#[poise::command(
    prefix_command,
    subcommand_required,
    guild_only,
    subcommands("add", "remove")
)]
pub async fn role(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// add a self-assignable role from list.
#[tracing::instrument(skip_all)]
#[poise::command(prefix_command)]
pub async fn add(ctx: Context<'_>) -> Result<(), Error> {
    let roles = sqlx::query_as!(
        Role,
        r#"
            SELECT
                id,
                name,
                role_id
            FROM roles
            ORDER BY id
            LIMIT 10
            OFFSET 0;
        "#
    )
    .fetch_all(&ctx.data().db)
    .await?;

    if !roles.is_empty() {
        let data: Vec<CreateSelectMenuOption> = roles
            .iter()
            .map(|role| CreateSelectMenuOption::new(&role.name, &role.role_id))
            .collect();

        let mut roles_str = String::new();

        for (idx, role) in roles.iter().enumerate() {
            roles_str = roles_str + &format!("{}. <@&{}>\n", idx, role.role_id);
        }

        let msg = ctx
            .send(
                poise::CreateReply::default()
                    .reply(true)
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .components(vec![CreateActionRow::SelectMenu(CreateSelectMenu::new(
                        "role_menu",
                        CreateSelectMenuKind::String { options: data },
                    ))])
                    .embed(CreateEmbed::default().field(
                        "list of self-assignable roles",
                        roles_str,
                        false,
                    )),
            )
            .await?;

        while let Some(mci) = ComponentInteractionCollector::new(ctx)
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .timeout(std::time::Duration::from_secs(120))
            .await
        {
            let mut msg = mci.message.clone();

            if let ComponentInteractionDataKind::StringSelect { values: roles } =
                mci.data.kind.clone()
            {
                let member = ctx
                    .http()
                    .get_member(ctx.guild_id().unwrap(), ctx.author().id)
                    .await?;

                if member
                    .roles
                    .contains(&RoleId::new(roles[0].parse::<u64>().unwrap()))
                {
                    msg.edit(
                        ctx,
                        EditMessage::new()
                            .content(format!("you already have <@&{}>!", roles[0]))
                            .components(vec![])
                            .embeds(vec![]),
                    )
                    .await?;

                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    break;
                }

                member
                    .add_role(ctx, roles[0].parse::<u64>().unwrap())
                    .await?;

                msg.edit(
                    ctx,
                    EditMessage::new()
                        .content(format!("added <@&{}> to your account.", roles[0]))
                        .components(vec![])
                        .embeds(vec![]),
                )
                .await?;

                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                break;
            }
        }

        msg.delete(ctx).await?;
    } else {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("no self-assignable roles were configured!"),
        )
        .await?;
    }

    Ok(())
}

/// remove a self-assignable role from your role list.
#[tracing::instrument(skip_all)]
#[poise::command(prefix_command)]
pub async fn remove(ctx: Context<'_>) -> Result<(), Error> {
    let roles = sqlx::query_as!(
        Role,
        r#"
            SELECT
                id,
                name,
                role_id
            FROM roles
            ORDER BY id
            LIMIT 10
            OFFSET 0;
        "#
    )
    .fetch_all(&ctx.data().db)
    .await?;

    if !roles.is_empty() {
        let roles_set: HashSet<RoleId> = HashSet::from_iter(
            roles
                .iter()
                .map(|role| RoleId::new(role.role_id.parse::<u64>().unwrap())),
        );

        let mut roles_str = String::new();

        let member = ctx
            .http()
            .get_member(ctx.guild_id().unwrap(), ctx.author().id)
            .await?;

        let user_roles_set: HashSet<RoleId> = HashSet::from_iter(member.roles.iter().copied());

        let self_assignable_roles = roles_set.intersection(&user_roles_set);

        let data: Vec<CreateSelectMenuOption> = self_assignable_roles
            .clone()
            .map(|role| {
                let guild = ctx.guild().unwrap();
                let role = guild.roles.get(role).unwrap();

                CreateSelectMenuOption::new(&role.name, role.id.to_string())
            })
            .collect();

        for (idx, role) in self_assignable_roles.clone().enumerate() {
            roles_str = roles_str + &format!("{}. <@&{}>\n", idx, role);
        }

        if self_assignable_roles.count() == 0 {
            ctx.send(
                poise::CreateReply::default()
                    .reply(true)
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("you don't have any self-assignable roles!"),
            )
            .await?;
            return Ok(());
        }

        let msg = ctx
            .send(
                poise::CreateReply::default()
                    .reply(true)
                    .components(vec![CreateActionRow::SelectMenu(CreateSelectMenu::new(
                        "role_menu",
                        CreateSelectMenuKind::String { options: data },
                    ))])
                    .embed(CreateEmbed::default().field(
                        "list of your self-assignable roles",
                        roles_str,
                        false,
                    )),
            )
            .await?;

        while let Some(mci) = ComponentInteractionCollector::new(ctx)
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .timeout(std::time::Duration::from_secs(120))
            .await
        {
            let mut msg = mci.message.clone();

            if let ComponentInteractionDataKind::StringSelect { values: roles } =
                mci.data.kind.clone()
            {
                let member = ctx
                    .http()
                    .get_member(ctx.guild_id().unwrap(), ctx.author().id)
                    .await?;

                if member
                    .roles
                    .contains(&RoleId::new(roles[0].parse::<u64>().unwrap()))
                {
                    member
                        .remove_role(ctx, roles[0].parse::<u64>().unwrap())
                        .await?;

                    msg.edit(
                        ctx,
                        EditMessage::default()
                            .content(format!("i removed the role <@&{}> for you!", roles[0]))
                            .components(vec![])
                            .embeds(vec![]),
                    )
                    .await?;

                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    break;
                }

                msg.edit(
                    ctx,
                    EditMessage::new()
                        .content(format!(
                            "you don't seem to have the role <@&{}>...? how did you get here to \
                             begin with?",
                            roles[0]
                        ))
                        .components(vec![])
                        .embeds(vec![]),
                )
                .await?;

                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                break;
            }
        }

        msg.delete(ctx).await?;
    } else {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("no self-assignable roles were configured!"),
        )
        .await?;
    }

    Ok(())
}
