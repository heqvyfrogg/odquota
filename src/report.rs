//! Merge raw API responses into the CLI output document.
//!
//! Design rules that make the output trustworthy:
//!   * fields we did not obtain are omitted rather than filled with 0 or null,
//!   * the snapshot's model-independent windows are the base, and preflight
//!     only adds `usedCredits` / `policyId` / `durationSeconds` for the same
//!     window kind,
//!   * percentages are computed from the decimal strings via integer math so
//!     no floating point drift appears in the last digit.

use crate::model::{
    BillingOut, CodingWindow, ConsumptionOut, CreditsOut, EntitledModelOut, PermissionsOut,
    PlanOut, Preflight, PreflightOut, QuotaOut, Snapshot, SnapshotWindow, Summary, WalletGrantOut,
    WalletOut, WindowOut,
};

/// Percentage of `remaining` within `limit`, as a value rounded to 2 decimals.
/// Returns `None` when either side is missing, non-numeric, or the limit is 0.
pub fn remaining_percent(remaining: &str, limit: &str) -> Option<f64> {
    let rem = parse_credits(remaining)?;
    let lim = parse_credits(limit)?;
    if lim == 0 {
        return None;
    }
    // Integer arithmetic on i128 keeps the division exact for these magnitudes.
    let scaled = rem as i128 * 10_000 / lim as i128; // 100 * 100 for 2 decimals
    Some(scaled as f64 / 100.0)
}

fn parse_credits(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i64>().ok()
}

/// Seconds from `now` until `resets_at`. Negative values are preserved: a reset
/// time in the past is a fact worth reporting, not something to clamp to zero.
pub fn resets_in_seconds(
    resets_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<i64> {
    let raw = resets_at?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed = chrono::DateTime::parse_from_rfc3339(raw).ok()?;
    Some((parsed.with_timezone(&chrono::Utc) - now).num_seconds())
}

fn window_kind_from_duration(seconds: u64) -> String {
    // 5h and 7d are the only kinds Open Design publishes; keep the mapping
    // explicit so an unexpected duration is not silently folded into one.
    match seconds {
        18_000 => "5h".to_string(),
        604_800 => "7d".to_string(),
        other => format!("{other}s"),
    }
}

fn from_snapshot_window(w: &SnapshotWindow, now: chrono::DateTime<chrono::Utc>) -> WindowOut {
    let remaining_percent = match (&w.remaining_credits, &w.limit_credits) {
        (Some(r), Some(l)) => remaining_percent(r, l),
        _ => None,
    };
    let exhausted = w
        .remaining_credits
        .as_deref()
        .and_then(parse_credits)
        .map(|r| r == 0)
        .unwrap_or(false);

    WindowOut {
        kind: w.kind.clone(),
        duration_seconds: None,
        policy_id: None,
        limit_credits: w.limit_credits.clone(),
        used_credits: None,
        remaining_credits: w.remaining_credits.clone(),
        remaining_percent,
        exhausted,
        window_start: w.window_start.clone(),
        resets_at: w.resets_at.clone(),
        resets_in_seconds: resets_in_seconds(w.resets_at.as_deref(), now),
    }
}

/// Build the window list from the snapshot, then layer preflight details on top
/// for windows of the same kind. Preflight is advisory: if a kind is absent from
/// one source, the other value still stands on its own.
pub fn merge_windows(
    snapshot: Option<&Snapshot>,
    preflight: Option<&Preflight>,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<WindowOut> {
    let mut windows: Vec<WindowOut> = Vec::new();

    if let Some(snap) = snapshot {
        if let Some(list) = snap.design_plan_windows.as_ref() {
            for w in list {
                windows.push(from_snapshot_window(w, now));
            }
        }
    }

    if let Some(pre) = preflight {
        if let Some(plan) = pre.coding_plan.as_ref() {
            if let Some(coding_windows) = plan.windows.as_ref() {
                for cw in coding_windows {
                    let kind = cw
                        .duration_seconds
                        .map(window_kind_from_duration)
                        .unwrap_or_else(|| "unknown".to_string());
                    match windows.iter_mut().find(|w| w.kind == kind) {
                        Some(existing) => apply_preflight(existing, cw, now),
                        None => {
                            let mut created = WindowOut {
                                kind,
                                duration_seconds: cw.duration_seconds,
                                policy_id: cw.policy_id.clone(),
                                limit_credits: cw.limit_credits.clone(),
                                used_credits: cw.used_credits.clone(),
                                remaining_credits: cw.remaining_credits.clone(),
                                remaining_percent: match (&cw.remaining_credits, &cw.limit_credits)
                                {
                                    (Some(r), Some(l)) => remaining_percent(r, l),
                                    _ => None,
                                },
                                exhausted: false,
                                window_start: cw.window_start.clone(),
                                resets_at: cw.resets_at.clone(),
                                resets_in_seconds: resets_in_seconds(cw.resets_at.as_deref(), now),
                            };
                            created.exhausted = cw
                                .remaining_credits
                                .as_deref()
                                .and_then(parse_credits)
                                .map(|r| r == 0)
                                .unwrap_or(false);
                            windows.push(created);
                        }
                    }
                }
            }
        }
    }

    windows.sort_by_key(|w| w.duration_seconds.unwrap_or(u64::MAX));
    windows
}

fn apply_preflight(
    existing: &mut WindowOut,
    cw: &CodingWindow,
    now: chrono::DateTime<chrono::Utc>,
) {
    if existing.duration_seconds.is_none() {
        existing.duration_seconds = cw.duration_seconds;
    }
    if existing.policy_id.is_none() {
        existing.policy_id = cw.policy_id.clone();
    }
    if existing.used_credits.is_none() {
        existing.used_credits = cw.used_credits.clone();
    }
    // Prefer the preflight credits: they are per-window and refreshed on the
    // same request as the coverage decision, while snapshot values may lag.
    if let Some(v) = cw.remaining_credits.clone() {
        existing.remaining_credits = Some(v);
    }
    if let Some(v) = cw.limit_credits.clone() {
        existing.limit_credits = Some(v);
    }
    if let Some(v) = cw.window_start.clone() {
        if existing.window_start.is_none() {
            existing.window_start = Some(v);
        }
    }
    if let Some(v) = cw.resets_at.clone() {
        existing.resets_at = Some(v);
    }
    existing.remaining_percent = match (&existing.remaining_credits, &existing.limit_credits) {
        (Some(r), Some(l)) => remaining_percent(r, l),
        _ => None,
    };
    existing.exhausted = existing
        .remaining_credits
        .as_deref()
        .and_then(parse_credits)
        .map(|r| r == 0)
        .unwrap_or(false);
    existing.resets_in_seconds = resets_in_seconds(existing.resets_at.as_deref(), now);
}

pub fn build_quota(
    snapshot: Option<&Snapshot>,
    summary: Option<&Summary>,
    preflight: Option<&Preflight>,
    now: chrono::DateTime<chrono::Utc>,
) -> QuotaOut {
    let billing = snapshot
        .and_then(|s| s.billing.as_ref())
        .map(|b| BillingOut {
            billing_state: b.billing_state.clone(),
            plan_id: b.plan_id.clone(),
            billing_mode: snapshot.and_then(|s| s.billing_mode.clone()),
        });

    let plan = snapshot.and_then(|s| s.plan.as_ref()).map(|p| PlanOut {
        tier: p.tier.clone(),
        interval: p.interval.clone(),
        status: p.status.clone(),
        current_period_start: p.current_period_start.clone(),
        current_period_end: p.current_period_end.clone(),
        cancel_at_period_end: p.cancel_at_period_end,
    });

    let credits = summary.map(|s| CreditsOut {
        subscription_credits: s
            .balances
            .as_ref()
            .and_then(|b| b.subscription_credits.clone()),
        recharge_credits: s.balances.as_ref().and_then(|b| b.recharge_credits.clone()),
        total_available_credits: s
            .balances
            .as_ref()
            .and_then(|b| b.total_available_credits.clone()),
        credits_per_usd: s.credits_per_usd,
        subscription_credits_usd: s.subscription_credits_usd.clone(),
        recharge_credits_usd: s.recharge_credits_usd.clone(),
        total_available_credits_usd: s.total_available_credits_usd.clone(),
    });

    let wallet = snapshot.and_then(|s| s.wallet.as_ref()).map(|w| WalletOut {
        balance_usd: w.balance_usd.clone(),
        expires_at: w.expires_at.clone(),
        updated_at: w.updated_at.clone(),
        breakdown: snapshot
            .and_then(|s| s.wallet_breakdown.as_ref())
            .map(|grants| {
                grants
                    .iter()
                    .map(|g| WalletGrantOut {
                        kind: g.kind.clone(),
                        amount_usd: g.amount_usd.clone(),
                        expires_at: g.expires_at.clone(),
                        allowed_models: g
                            .restriction
                            .as_ref()
                            .and_then(|r| r.allowed_models.clone())
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    });

    let consumption = summary.map(|s| ConsumptionOut {
        usage_count: s.usage_count,
        today_consumed_usd: s.today_consumed_usd.clone(),
        total_consumed_usd: s.total_consumed_usd.clone(),
        total_recharged_usd: s.total_recharged_usd.clone(),
        recharge_count: s.recharge_count,
        updated_at: s.updated_at.clone(),
    });

    let permissions = snapshot
        .and_then(|s| s.permissions.as_ref())
        .map(|p| PermissionsOut {
            can_upgrade: p.can_upgrade,
            can_recharge: p.can_recharge,
            can_configure_auto_recharge: p.can_configure_auto_recharge,
            can_manage_subscription: p.can_manage_subscription,
        });

    let preflight_out = preflight.map(|p| PreflightOut {
        model_id: p.model_id.clone(),
        model_covered: p.model_covered,
        funding: p.funding.clone(),
        funding_check: p.funding_check.clone(),
        task_check: p.task_check.clone(),
        eligible: p.coding_plan.as_ref().and_then(|c| c.eligible),
        tier: p.coding_plan.as_ref().and_then(|c| c.tier.clone()),
    });

    QuotaOut {
        billing,
        plan,
        credits,
        wallet,
        windows: merge_windows(snapshot, preflight, now),
        consumption,
        permissions,
        preflight: preflight_out,
        entitled_models: None,
        auto_recharge: None,
    }
}

pub fn map_auto_recharge(a: &crate::model::AutoRecharge) -> crate::model::AutoRechargeOut {
    crate::model::AutoRechargeOut {
        enabled: a.enabled,
        lower_threshold_usd: a.lower_threshold_usd.clone(),
        topup_amount_usd: a.topup_amount_usd.clone(),
        monthly_limit_usd: a.monthly_limit_usd.clone(),
        monthly_limit_unlimited: a.monthly_limit_unlimited,
        payment_method_status: a.payment_method.as_ref().and_then(|p| p.status.clone()),
        month_succeeded_usd: a.month.as_ref().and_then(|m| m.succeeded_usd.clone()),
        month_in_flight_usd: a.month.as_ref().and_then(|m| m.in_flight_usd.clone()),
        has_last_failure: Some(a.last_failure.is_some()),
    }
}

pub fn map_catalog(c: &crate::model::ModelCatalog) -> Vec<crate::model::ModelOut> {
    c.data
        .iter()
        .map(|m| crate::model::ModelOut {
            id: m.id.clone(),
            name: m.name.clone(),
            cost: m.metadata.as_ref().and_then(|md| md.cost.clone()),
            capability: m.metadata.as_ref().and_then(|md| md.capability.clone()),
        })
        .collect()
}

pub fn map_entitled(list: &crate::model::LinkModelList) -> Vec<EntitledModelOut> {
    list.data
        .iter()
        .filter(|m| m.enabled.unwrap_or(true))
        .map(|m| EntitledModelOut {
            id: m.id.clone(),
            cost: m.metadata.as_ref().and_then(|md| md.cost.clone()),
            capability: m.metadata.as_ref().and_then(|md| md.capability.clone()),
            context_budget: m.metadata.as_ref().and_then(|md| md.context_budget),
            output_budget: m.metadata.as_ref().and_then(|md| md.output_budget),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-11-08T09:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn percent_uses_two_decimals() {
        assert_eq!(remaining_percent("47320", "50000"), Some(94.64));
        assert_eq!(remaining_percent("50000", "50000"), Some(100.0));
        assert_eq!(remaining_percent("0", "50000"), Some(0.0));
        assert_eq!(remaining_percent("1", "3"), Some(33.33));
    }

    #[test]
    fn percent_rejects_missing_or_zero_limit() {
        assert_eq!(remaining_percent("100", "0"), None);
        assert_eq!(remaining_percent("", "100"), None);
        assert_eq!(remaining_percent("abc", "100"), None);
        assert_eq!(remaining_percent("100", ""), None);
    }

    #[test]
    fn percent_is_not_rounded_to_integer() {
        // 9999/10000 must not collapse to 100.0
        assert_eq!(remaining_percent("9999", "10000"), Some(99.99));
    }

    #[test]
    fn resets_seconds_preserves_past_values() {
        let past = resets_in_seconds(Some("2026-11-08T08:00:00Z"), now());
        assert_eq!(past, Some(-3600));
        let future = resets_in_seconds(Some("2026-11-08T10:00:00Z"), now());
        assert_eq!(future, Some(3600));
        assert_eq!(resets_in_seconds(None, now()), None);
        assert_eq!(resets_in_seconds(Some("garbage"), now()), None);
    }

    #[test]
    fn maps_duration_to_kind() {
        assert_eq!(window_kind_from_duration(18_000), "5h");
        assert_eq!(window_kind_from_duration(604_800), "7d");
        assert_eq!(window_kind_from_duration(60), "60s");
    }

    #[test]
    fn preflight_fills_snapshot_windows() {
        let snapshot: Snapshot = serde_json::from_str(
            r#"{
              "workspaceId":"ws","workspaceMemberId":"wm",
              "designPlanWindows":[
                {"kind":"5h","windowStart":"2026-11-08T06:51:55.310Z","resetsAt":"2026-11-08T11:51:55.310Z","remainingCredits":"47320","limitCredits":"50000"},
                {"kind":"7d","windowStart":"2026-11-07T10:51:55.310Z","resetsAt":"2026-11-14T10:51:55.310Z","remainingCredits":"116039","limitCredits":"150000"}
              ]
            }"#,
        )
        .unwrap();
        let preflight: Preflight = serde_json::from_str(
            r#"{
              "modelId":"deepseek-v4.1-flash",
              "codingPlan":{"eligible":true,"tier":"go","windows":[
                {"policyId":"p5","durationSeconds":18000,"resetMode":"anchored_recurring","usedCredits":"2680","limitCredits":"50000","remainingCredits":"47320","windowStart":"2026-11-08T06:51:55.310Z","resetsAt":"2026-11-08T11:51:55.310Z"},
                {"policyId":"p7","durationSeconds":604800,"resetMode":"anchored_recurring","usedCredits":"33961","limitCredits":"150000","remainingCredits":"116039","windowStart":"2026-11-07T10:51:55.310Z","resetsAt":"2026-11-14T10:51:55.310Z"}
              ]}
            }"#,
        )
        .unwrap();
        let w = merge_windows(Some(&snapshot), Some(&preflight), now());
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].kind, "5h");
        assert_eq!(w[0].duration_seconds, Some(18_000));
        assert_eq!(w[0].policy_id.as_deref(), Some("p5"));
        assert_eq!(w[0].used_credits.as_deref(), Some("2680"));
        assert_eq!(w[0].remaining_percent, Some(94.64));
        assert_eq!(w[0].resets_in_seconds, Some(10_315));
        assert_eq!(w[1].kind, "7d");
    }

    #[test]
    fn snapshot_only_windows_have_no_preflight_fields() {
        let snapshot: Snapshot = serde_json::from_str(
            r#"{"workspaceId":"ws","workspaceMemberId":"wm",
                "designPlanWindows":[{"kind":"5h","remainingCredits":"0","limitCredits":"50000"}]}"#,
        )
        .unwrap();
        let w = merge_windows(Some(&snapshot), None, now());
        assert_eq!(w.len(), 1);
        assert!(w[0].used_credits.is_none());
        assert!(w[0].policy_id.is_none());
        assert!(w[0].duration_seconds.is_none());
        assert!(w[0].exhausted);
        assert_eq!(w[0].remaining_percent, Some(0.0));
    }

    #[test]
    fn missing_fields_are_omitted_not_zeroed() {
        let snapshot: Snapshot =
            serde_json::from_str(r#"{"workspaceId":"ws","workspaceMemberId":"wm"}"#).unwrap();
        let q = build_quota(Some(&snapshot), None, None, now());
        let json = serde_json::to_value(&q).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("credits"));
        assert!(!obj.contains_key("wallet"));
        assert!(!obj.contains_key("consumption"));
        assert!(obj.contains_key("windows"));
        assert_eq!(obj["windows"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn empty_optional_strings_are_not_reported() {
        // A wallet with no balance must not serialize balanceUsd at all.
        let snapshot: Snapshot =
            serde_json::from_str(r#"{"workspaceId":"ws","workspaceMemberId":"wm","wallet":{}}"#)
                .unwrap();
        let q = build_quota(Some(&snapshot), None, None, now());
        let json = serde_json::to_value(&q).unwrap();
        let wallet = &json["wallet"];
        assert!(wallet.is_object());
        assert!(wallet.get("balanceUsd").is_none());
    }
}
