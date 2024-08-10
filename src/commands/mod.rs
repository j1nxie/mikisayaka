use crate::Context;

pub mod help;
pub mod status;

pub(crate) fn get_bot_avatar(ctx: Context<'_>) -> String {
    ctx.cache().current_user().avatar_url().unwrap_or_default()
}
