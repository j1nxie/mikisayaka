use poise::serenity_prelude::{
    ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateEmbed,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditMessage, RoleId,
};
use sea_orm::{EntityTrait, PaginatorTrait};

use crate::{models::roles, Context, Error};

/// commands related to self-assignable roles.
#[poise::command(
    slash_command,
    prefix_command,
    subcommand_required,
    guild_only,
    subcommands("add", "remove")
)]
pub async fn role(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// add a self-assignable role from list.
#[poise::command(slash_command, prefix_command)]
pub async fn add(ctx: Context<'_>) -> Result<(), Error> {
    let mut db_roles_find = roles::Entity::find().paginate(&ctx.data().db, 10);

    let db_roles = db_roles_find.fetch_and_next().await?;

    if let Some(roles) = db_roles {
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

                    std::thread::sleep(std::time::Duration::from_secs(10));
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

                std::thread::sleep(std::time::Duration::from_secs(10));
                break;
            }
        }

        msg.delete(ctx).await?;
    } else {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .content("no self-assignable roles were configured!"),
        )
        .await?;
    }

    Ok(())
}

/// remove a self-assignable role from your role list.
#[poise::command(slash_command, prefix_command)]
pub async fn remove(ctx: Context<'_>) -> Result<(), Error> {
    // let mut db_roles_find = roles::Entity::find().paginate(&ctx.data().db, 10);

    // let db_roles = db_roles_find.fetch_and_next().await?;

    // if let Some(roles) = db_roles {
    //     let data: Vec<CreateSelectMenuOption> = roles
    //         .iter()
    //         .map(|role| CreateSelectMenuOption::new(&role.name, &role.role_id))
    //         .collect();

    //     let mut roles_str = String::new();

    //     let member = ctx
    //         .http()
    //         .get_member(ctx.guild_id().unwrap(), ctx.author().id)
    //         .await?;

    //     for (idx, role) in member.roles.iter().enumerate() {
    //         roles_str = roles_str + &format!("{}. <@&{}>\n", idx, role);
    //     }

    //     let msg = ctx
    //         .send(
    //             poise::CreateReply::default()
    //                 .components(vec![CreateActionRow::SelectMenu(CreateSelectMenu::new(
    //                     "role_menu",
    //                     CreateSelectMenuKind::String { options: data },
    //                 ))])
    //                 .embed(CreateEmbed::default().field(
    //                     "list of your self-assignable roles",
    //                     roles_str,
    //                     false,
    //                 )),
    //         )
    //         .await?;

    //     while let Some(mci) = ComponentInteractionCollector::new(ctx)
    //         .author_id(ctx.author().id)
    //         .channel_id(ctx.channel_id())
    //         .timeout(std::time::Duration::from_secs(120))
    //         .await
    //     {
    //         let mut msg = mci.message.clone();

    //         if let ComponentInteractionDataKind::StringSelect { values: roles } =
    //             mci.data.kind.clone()
    //         {
    //             let member = ctx
    //                 .http()
    //                 .get_member(ctx.guild_id().unwrap(), ctx.author().id)
    //                 .await?;

    //             if member
    //                 .roles
    //                 .contains(&RoleId::new(roles[0].parse::<u64>().unwrap()))
    //             {
    //                 msg.edit(
    //                     ctx,
    //                     EditMessage::new()
    //                         .content(format!("you already have <@&{}>!", roles[0]))
    //                         .components(vec![])
    //                         .embeds(vec![]),
    //                 )
    //                 .await?;

    //                 std::thread::sleep(std::time::Duration::from_secs(10));
    //                 break;
    //             }

    //             member
    //                 .add_role(ctx, roles[0].parse::<u64>().unwrap())
    //                 .await?;

    //             msg.edit(
    //                 ctx,
    //                 EditMessage::new()
    //                     .content(format!("added <@&{}> to your account.", roles[0]))
    //                     .components(vec![])
    //                     .embeds(vec![]),
    //             )
    //             .await?;

    //             std::thread::sleep(std::time::Duration::from_secs(10));
    //             break;
    //         }
    //     }

    //     msg.delete(ctx).await?;
    // } else {
    //     ctx.send(
    //         poise::CreateReply::default().content("no self-assignable roles were configured!"),
    //     )
    //     .await?;
    // }

    ctx.reply("this command is not yet implemented.").await?;

    Ok(())
}
