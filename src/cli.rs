//! Command line surface.

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "odquota",
    version,
    about = "Open Design のアカウントクオータを JSON で出力する",
    long_about = "保存済みのアカウントについて Vela 制御プレーンの課金 API を叩き、\
                  残クレジット・ウォレット・プランを 1 本の JSON にまとめて stdout へ出す。\
                  認証情報は DPAPI で暗号化して保存する。"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// アカウント（認証情報）の登録・一覧・削除
    #[command(subcommand)]
    Account(AccountCommand),
    /// 保存済みアカウントのクオータを JSON で出力
    Quota(QuotaArgs),
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// ~/.amr/config.json から認証情報を取り込む
    ImportAmr(ImportAmrArgs),
    /// 鍵を標準入力または環境変数から登録する
    Add(AddArgs),
    /// 保存済みアカウントの一覧（鍵は出さない）
    List(ListArgs),
    /// アカウントを保存から削除する
    Remove(RemoveArgs),
}

#[derive(Debug, Args)]
pub struct ImportAmrArgs {
    /// 取り込むプロファイル名
    #[arg(long, default_value = "prod")]
    pub profile: String,
    /// 保存時のラベル（省略時はプロファイル名）
    #[arg(long)]
    pub label: Option<String>,
    /// 課金スコープのワークスペース ID（省略時は実行時に解決）
    #[arg(long)]
    pub workspace: Option<String>,
    /// 同じラベルが既にある場合に上書きする
    #[arg(long)]
    pub force: bool,
    /// 設定ファイルのパスを明示する
    #[arg(long)]
    pub config: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// 保存時のラベル
    #[arg(long)]
    pub label: String,
    /// controlKey が入っている環境変数名（値そのものは引数に取らない）
    #[arg(long = "control-key-env")]
    pub control_key_env: Option<String>,
    /// runtimeKey が入っている環境変数名
    #[arg(long = "runtime-key-env")]
    pub runtime_key_env: Option<String>,
    /// 標準入力から controlKey を読む（--control-key-env 指定時は使わない）
    #[arg(long)]
    pub stdin: bool,
    /// API のベース URL
    #[arg(long, default_value = crate::amr::DEFAULT_API_URL)]
    pub api_url: String,
    /// Link のベース URL
    #[arg(long, default_value = crate::amr::DEFAULT_LINK_URL)]
    pub link_url: String,
    /// 課金スコープのワークスペース ID
    #[arg(long)]
    pub workspace: Option<String>,
    /// 同じラベルが既にある場合に上書きする
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 整形して出力する
    #[arg(long)]
    pub pretty: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// 削除するラベル
    #[arg(long)]
    pub label: String,
}

#[derive(Debug, Args)]
pub struct QuotaArgs {
    /// 対象ラベル（省略時は保存済みの全アカウント）
    #[arg(long)]
    pub label: Option<String>,
    /// 保存済みの全アカウントを対象にする（既定）
    #[arg(long)]
    pub all: bool,
    /// このモデル ID で billing/preflight も取得する
    #[arg(long)]
    pub model: Option<String>,
    /// Link から権利付与済みモデル一覧も取得する
    #[arg(long)]
    pub entitled: bool,
    /// 到達できる読み取りエンドポイントの一覧（HTTP ステータス付き）を含める
    #[arg(long)]
    pub surfaces: bool,
    /// 全モデルカタログ（46 件）を含める
    #[arg(long)]
    pub catalog: bool,
    /// 整形して出力する
    #[arg(long)]
    pub pretty: bool,
}
