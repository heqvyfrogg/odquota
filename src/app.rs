//! コマンド実行の入口。
//!
//! Dispatch to the account commands or the quota report; each command module
//! owns its own argument handling so this file stays a thin router.

mod account;
mod quota;

pub use account::run_account;
pub use quota::run_quota;

use crate::cli::{Cli, Command};
use crate::error::AppError;

pub fn run(cli: Cli) -> Result<i32, AppError> {
    match cli.command {
        Command::Account(cmd) => run_account(cmd),
        Command::Quota(args) => run_quota(args),
    }
}
