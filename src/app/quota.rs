//! The `quota` command: fetch and merge the billing API into one JSON report.

use chrono::Utc;

use crate::api::Api;
use crate::app::account::storage_info;
use crate::cli::QuotaArgs;
use crate::error::{AppError, EXIT_OK, EXIT_PARTIAL};
use crate::model::{
    AccountReport, Identity, QuotaOut, Report, WorkspaceOut, WorkspaceSource, OUTPUT_SCHEMA_VERSION,
};
use crate::output::print_json;
use crate::report;
use crate::store::{AccountEntry, Store};

pub fn run_quota(args: QuotaArgs) -> Result<i32, AppError> {
    let store = Store::resolve()?;
    let doc = store.load()?;

    let selected: Vec<&AccountEntry> = match args.label.as_deref() {
        Some(label) => {
            let entry = doc.find(label).ok_or_else(|| {
                AppError::usage(format!("ラベル \"{label}\" は保存されていません"))
            })?;
            vec![entry]
        }
        None => doc.accounts.iter().collect(),
    };

    if selected.is_empty() {
        return Err(AppError::empty(
            "保存済みアカウントがありません。先に `odquota account import-amr` か `odquota account add` を実行してください。",
        ));
    }

    let api = Api::new().map_err(|f| AppError::store(f.message))?;
    let generated_at = Utc::now();

    let mut accounts = Vec::with_capacity(selected.len());
    for entry in selected {
        accounts.push(build_account_report(&api, entry, &args));
    }

    // Exit 1 means the run did not satisfy everything the caller asked for,
    // including a missing optional stage. ok still tells the exact truth per
    // account: a row with ok=true, complete=false produced a real quota but
    // lacks one of the requested extras.
    let any_incomplete = accounts.iter().any(|a| !a.complete);
    let out = Report {
        schema_version: OUTPUT_SCHEMA_VERSION,
        generated_at: generated_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        storage: storage_info(&store),
        accounts,
    };
    print_json(&out, args.pretty)?;
    Ok(if any_incomplete {
        EXIT_PARTIAL
    } else {
        EXIT_OK
    })
}

fn build_account_report(api: &Api, entry: &AccountEntry, args: &QuotaArgs) -> AccountReport {
    let now = Utc::now();
    let mut errors = Vec::new();
    let mut workspace_id = entry.workspace_id.clone();
    let mut workspace_info: Option<crate::model::WorkspaceItem> = None;

    // Resolve the workspace when the store did not pin one.
    match api.list_workspaces(&entry.api_url, &entry.control_key) {
        Ok(list) => {
            if workspace_id.is_none() {
                let default = list
                    .items
                    .iter()
                    .find(|w| w.is_default == Some(true))
                    .or_else(|| list.items.first());
                if let Some(w) = default {
                    workspace_id = Some(w.workspace_id.clone());
                    workspace_info = Some(workspace_item(w));
                } else {
                    errors.push(crate::model::StageError::new(
                        "workspaces",
                        "ワークスペースが 1 件も返りませんでした",
                    ));
                }
            } else if let Some(w) = list
                .items
                .iter()
                .find(|w| Some(w.workspace_id.as_str()) == workspace_id.as_deref())
            {
                workspace_info = Some(workspace_item(w));
            }
        }
        Err(f) => {
            // A missing workspace list is only fatal when we have no pinned id.
            if workspace_id.is_none() {
                return AccountReport::failed(&entry.label, vec![f.into_stage_error("workspaces")]);
            }
            errors.push(f.into_stage_error("workspaces"));
        }
    }

    let ws = match workspace_id {
        Some(v) => v,
        None => {
            errors.push(crate::model::StageError::new(
                "workspaces",
                "ワークスペース ID を解決できませんでした",
            ));
            return AccountReport::failed(&entry.label, errors);
        }
    };

    let mut surfaces: Vec<crate::model::EndpointStatus> = Vec::new();

    let identity = match api.me(&entry.api_url, &entry.control_key) {
        Ok(me) => Some(Identity {
            user_id: me.id,
            auth_user_id: me.auth_user_id,
            email: me.email,
            name: me.name,
            avatar_url: me.image,
            status: me.status,
            plan: me.plan,
            balance_usd: me.balance_usd,
        }),
        Err(f) => {
            errors.push(f.into_stage_error("me"));
            None
        }
    };

    let snapshot = match api.snapshot(&entry.api_url, &entry.control_key, &ws) {
        Ok(s) => Some(s),
        Err(f) => {
            errors.push(f.into_stage_error("billing_snapshot"));
            None
        }
    };

    let auto_recharge = match api.auto_recharge(&entry.api_url, &entry.control_key, &ws) {
        Ok(a) => {
            surfaces.push(crate::model::EndpointStatus {
                path: format!("/api/v1/workspaces/{ws}/auto-recharge"),
                http_status: Some(200),
                error_code: None,
            });
            Some(a)
        }
        Err(f) => {
            surfaces.push(crate::model::EndpointStatus {
                path: format!("/api/v1/workspaces/{ws}/auto-recharge"),
                http_status: f.http_status,
                error_code: f.code.clone(),
            });
            None
        }
    };

    let summary = match api.summary(&entry.api_url, &entry.control_key, &ws) {
        Ok(s) => Some(s),
        Err(f) => {
            errors.push(f.into_stage_error("billing_summary"));
            None
        }
    };

    // Preflight decides model coverage, not the base quota, so its failure is
    // reported but does not fail the account.
    let preflight = match args.model.as_deref() {
        Some(model) => match api.preflight(&entry.api_url, &entry.control_key, &ws, model) {
            Ok(p) => Some(p),
            Err(f) => {
                errors.push(f.into_stage_error("billing_preflight"));
                None
            }
        },
        None => None,
    };

    let entitled = if args.entitled {
        match entry.runtime_key.as_deref() {
            Some(rk) => match api.entitled_models(&entry.link_url, rk, &ws) {
                Ok(list) => Some(list),
                Err(f) => {
                    errors.push(f.into_stage_error("entitled_models"));
                    None
                }
            },
            None => {
                errors.push(crate::model::StageError::new(
                    "entitled_models",
                    "runtimeKey が未登録のため Link のモデル一覧を取得できません",
                ));
                None
            }
        }
    } else {
        None
    };

    // If neither quota source answered, the account is a failure: emitting an
    // empty quota object would look like a successful read of zero credits.
    if snapshot.is_none() && summary.is_none() {
        return AccountReport::failed(&entry.label, errors);
    }

    let mut quota: QuotaOut =
        report::build_quota(snapshot.as_ref(), summary.as_ref(), preflight.as_ref(), now);
    if let Some(list) = entitled.as_ref() {
        quota.entitled_models = Some(report::map_entitled(list));
    }
    if let Some(a) = auto_recharge.as_ref() {
        quota.auto_recharge = Some(report::map_auto_recharge(a));
    }

    // Record which endpoints exist for this account, so a reader can tell
    // "not requested" from "denied" from "empty".
    if args.surfaces {
        surfaces.extend(surface_probe(api, entry, &ws));
        // The auto-recharge fetch above already recorded its own status; keep
        // only the first entry per path so the list is a map, not a log.
        let mut seen = std::collections::BTreeSet::new();
        surfaces.retain(|s| seen.insert(s.path.clone()));
        surfaces.sort_by(|a, b| a.path.cmp(&b.path));
    }

    let catalog: Option<Vec<crate::model::ModelOut>> = if args.catalog {
        match api.model_catalog(&entry.api_url, &entry.control_key) {
            Ok(c) => Some(report::map_catalog(&c)),
            Err(f) => {
                errors.push(f.into_stage_error("model_catalog"));
                None
            }
        }
    } else {
        None
    };

    let workspace_out = WorkspaceOut {
        workspace_id: ws.clone(),
        workspace_member_id: workspace_info
            .as_ref()
            .map(|w| w.workspace_member_id.clone())
            .or_else(|| snapshot.as_ref().map(|s| s.workspace_member_id.clone()))
            .unwrap_or_default(),
        name: workspace_info
            .as_ref()
            .and_then(|w| w.workspace_name.clone()),
        workspace_type: workspace_info
            .as_ref()
            .and_then(|w| w.workspace_type.clone()),
        role: workspace_info.as_ref().and_then(|w| w.role.clone()),
        is_default: workspace_info.as_ref().and_then(|w| w.is_default),
        member_count: workspace_info
            .as_ref()
            .and_then(|w| w.member_count)
            .or_else(|| snapshot.as_ref().and_then(|s| s.member_count)),
        icon_key: workspace_info
            .as_ref()
            .and_then(|w| w.workspace_icon_key.clone()),
        member_status: workspace_info
            .as_ref()
            .and_then(|w| w.member_status.clone()),
        lifecycle_state: workspace_info
            .as_ref()
            .and_then(|w| w.lifecycle_state.clone()),
        owner_name: snapshot
            .as_ref()
            .and_then(|s| s.owner.as_ref())
            .and_then(|o| o.display_name.clone()),
        source: if entry.workspace_id.is_some() {
            WorkspaceSource::Stored
        } else {
            WorkspaceSource::Resolved
        },
    };

    AccountReport {
        label: entry.label.clone(),
        ok: true,
        complete: errors.is_empty(),
        fetched_at: Some(now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        identity,
        workspace: Some(workspace_out),
        quota: Some(quota),
        surfaces,
        catalog,
        errors,
    }
}

/// Probe the read-only surfaces that exist for this account. Status is kept
/// verbatim: 403 and 503 are facts about the server, not reasons to pretend the
/// endpoint is absent.
fn surface_probe(api: &Api, entry: &AccountEntry, ws: &str) -> Vec<crate::model::EndpointStatus> {
    let mut out = Vec::new();
    for path in [
        "/api/v1/me".to_string(),
        "/api/v1/workspaces".to_string(),
        "/api/v1/team-projects".to_string(),
        "/api/v1/collab/members".to_string(),
        "/api/v1/resources".to_string(),
        "/api/v1/media/models".to_string(),
        "/api/v1/models".to_string(),
        "/api/v1/billing/summary".to_string(),
        format!("/api/v1/workspaces/{ws}/billing/snapshot"),
        format!("/api/v1/workspaces/{ws}/wallet/balance"),
        format!("/api/v1/workspaces/{ws}/auto-recharge"),
        format!("/api/v1/workspaces/{ws}/billing/ledger"),
        format!("/api/v1/workspaces/{ws}/billing/usage"),
        format!("/api/v1/workspaces/{ws}/members"),
        format!("/api/v1/workspaces/{ws}/billing/team-subscription/catalog"),
    ] {
        out.push(api.probe(&entry.api_url, &entry.control_key, &path));
    }
    out
}

fn workspace_item(w: &crate::model::WorkspaceItem) -> crate::model::WorkspaceItem {
    crate::model::WorkspaceItem {
        workspace_id: w.workspace_id.clone(),
        workspace_member_id: w.workspace_member_id.clone(),
        workspace_icon_key: w.workspace_icon_key.clone(),
        workspace_name: w.workspace_name.clone(),
        workspace_type: w.workspace_type.clone(),
        role: w.role.clone(),
        is_default: w.is_default,
        member_count: w.member_count,
        member_status: w.member_status.clone(),
        lifecycle_state: w.lifecycle_state.clone(),
    }
}
