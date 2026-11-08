//! Account subcommands: import from `~/.amr/config.json`, manual add, list, remove.

use std::io::{self, BufRead};

use chrono::Utc;

use crate::amr;
use crate::cli::{AccountCommand, ImportAmrArgs};
use crate::error::{AppError, EXIT_OK};
use crate::model::{AccountListItem, AccountListOutput, StorageInfo, OUTPUT_SCHEMA_VERSION};
use crate::output::print_json;
use crate::store::{AccountEntry, Store};

pub fn run_account(cmd: AccountCommand) -> Result<i32, AppError> {
    let store = Store::resolve()?;
    match cmd {
        AccountCommand::ImportAmr(args) => import_amr(&store, args),
        AccountCommand::Add(args) => add(&store, args),
        AccountCommand::List(args) => list(&store, args.pretty),
        AccountCommand::Remove(args) => remove(&store, &args.label),
    }
}

fn import_amr(store: &Store, args: ImportAmrArgs) -> Result<i32, AppError> {
    let path = match args.config {
        Some(p) => std::path::PathBuf::from(p),
        None => amr::default_config_path()?,
    };
    let label = args.label.unwrap_or_else(|| args.profile.clone());
    let entry = amr::read_profile(&path, &args.profile, &label, args.workspace)?;
    upsert(store, entry, args.force)?;
    eprintln!(
        "odquota: ラベル \"{}\" を取り込みました（保存先 {}）",
        label,
        store.path().display()
    );
    Ok(EXIT_OK)
}

fn add(store: &Store, args: crate::cli::AddArgs) -> Result<i32, AppError> {
    // Keys are read from an environment variable name or stdin; never from argv,
    // because process arguments are visible to other local processes.
    let control_key = match args.control_key_env.as_deref() {
        Some(var) => std::env::var(var)
            .map_err(|_| AppError::usage(format!("環境変数 {var} が未設定です")))?,
        None => {
            if !args.stdin {
                return Err(AppError::usage(
                    "--control-key-env か --stdin のどちらかを指定してください（鍵を引数には取りません）",
                ));
            }
            read_secret_from_stdin("controlKey")?
        }
    };
    if control_key.trim().is_empty() {
        return Err(AppError::usage("controlKey が空です"));
    }

    let runtime_key = match args.runtime_key_env.as_deref() {
        Some(var) => Some(
            std::env::var(var)
                .map_err(|_| AppError::usage(format!("環境変数 {var} が未設定です")))?,
        ),
        None => None,
    }
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty());

    let entry = AccountEntry {
        label: args.label.clone(),
        control_key: control_key.trim().to_string(),
        runtime_key,
        api_url: args.api_url.trim_end_matches('/').to_string(),
        link_url: args.link_url.trim_end_matches('/').to_string(),
        workspace_id: args
            .workspace
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        api_url_source: "manual".to_string(),
        added_at: Utc::now().to_rfc3339(),
        source: "manual".to_string(),
    };
    upsert(store, entry, args.force)?;
    eprintln!(
        "odquota: ラベル \"{}\" を登録しました（保存先 {}）",
        args.label,
        store.path().display()
    );
    Ok(EXIT_OK)
}

fn list(store: &Store, pretty: bool) -> Result<i32, AppError> {
    let doc = store.load()?;
    let out = AccountListOutput {
        schema_version: OUTPUT_SCHEMA_VERSION,
        storage: storage_info(store),
        accounts: doc
            .accounts
            .iter()
            .map(|a| AccountListItem {
                label: a.label.clone(),
                api_url: a.api_url.clone(),
                link_url: a.link_url.clone(),
                workspace_id: a.workspace_id.clone(),
                has_runtime_key: a.runtime_key.is_some(),
                added_at: a.added_at.clone(),
                source: a.source.clone(),
            })
            .collect(),
    };
    print_json(&out, pretty)?;
    Ok(EXIT_OK)
}

fn remove(store: &Store, label: &str) -> Result<i32, AppError> {
    let mut doc = store.load()?;
    let before = doc.accounts.len();
    doc.accounts.retain(|a| a.label != label);
    if doc.accounts.len() == before {
        return Err(AppError::usage(format!(
            "ラベル \"{label}\" は保存されていません"
        )));
    }
    store.save(&doc)?;
    eprintln!("odquota: ラベル \"{label}\" を削除しました");
    Ok(EXIT_OK)
}

fn upsert(store: &Store, entry: AccountEntry, force: bool) -> Result<(), AppError> {
    // Without --force, an existing label is an error rather than an overwrite:
    // a second import must not silently destroy a key that is still in use.
    let mut doc = store.load()?;
    if !force && doc.find(&entry.label).is_some() {
        return Err(AppError::usage(format!(
            "ラベル \"{}\" は既に保存されています（上書きするなら --force）",
            entry.label
        )));
    }
    doc.accounts.retain(|a| a.label != entry.label);
    doc.accounts.push(entry);
    store.save(&doc)?;
    Ok(())
}

fn read_secret_from_stdin(what: &str) -> Result<String, AppError> {
    eprintln!("odquota: {what} を標準入力から読みます（改行で確定）");
    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| AppError::usage(format!("標準入力を読めません: {e}")))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

pub fn storage_info(store: &Store) -> StorageInfo {
    StorageInfo {
        path: store.path().display().to_string(),
        encrypted: store.encrypted(),
        protection: store.protection().name().to_string(),
    }
}
