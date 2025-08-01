use crate::Context;

pub mod fluff;
pub mod gas_prices;
pub mod help;
pub mod manga;
pub mod quote;
pub mod role;
pub mod status;
pub mod zenless;

pub(crate) fn get_bot_avatar(ctx: Context<'_>) -> String {
    ctx.cache().current_user().avatar_url().unwrap_or_default()
}
