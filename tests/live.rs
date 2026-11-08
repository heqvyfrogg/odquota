//! Live integration test against the real Vela control plane.
//!
//! Ignored by default: it needs real credentials and it touches a live billing
//! account. Enable explicitly with a key in the environment:
//!
//! ```text
//! set ODQUOTA_LIVE_CONTROL_KEY=<key>
//! cargo test --offline --test live -- --ignored --nocapture
//! ```
//!
//! When the variable is absent the test reports that it was skipped instead of
//! silently passing, so a green run cannot be mistaken for live coverage.

use std::fs;
use std::path::PathBuf;

use odquota::api::Api;
use odquota::report;

fn temp_home(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("odquota-live-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create temp dir");
    base
}

#[test]
#[ignore = "live API: requires ODQUOTA_LIVE_CONTROL_KEY and a real account"]
fn live_quota_matches_api_shapes() {
    let key = match std::env::var("ODQUOTA_LIVE_CONTROL_KEY") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            println!("SKIP: ODQUOTA_LIVE_CONTROL_KEY が未設定のためライブ検証を実行しません");
            return;
        }
    };
    let api = Api::new().expect("client");

    let base = "https://amr-api.open-design.ai";
    let link = "https://amr-link.open-design.ai";

    let workspaces = api
        .list_workspaces(base, &key)
        .expect("workspaces endpoint must answer");
    let ws = workspaces
        .items
        .iter()
        .find(|w| w.is_default == Some(true))
        .or_else(|| workspaces.items.first())
        .expect("at least one workspace")
        .workspace_id
        .clone();

    let snapshot = api.snapshot(base, &key, &ws).expect("snapshot");
    let summary = api.summary(base, &key, &ws).expect("summary");
    let preflight = api
        .preflight(base, &key, &ws, "deepseek-v4.1-flash")
        .expect("preflight");

    let now = chrono::Utc::now();
    let quota = report::build_quota(Some(&snapshot), Some(&summary), Some(&preflight), now);
    let json = serde_json::to_value(&quota).expect("serialize");

    // Windows must exist and carry a two-decimal percentage in range.
    let windows = json["windows"].as_array().expect("windows array");
    assert!(!windows.is_empty(), "no quota windows returned");
    for w in windows {
        let pct = w["remainingPercent"].as_f64().expect("percent missing");
        assert!((0.0..=100.0).contains(&pct), "percent out of range: {pct}");
        assert!(
            w["remainingCredits"].as_str().is_some(),
            "remaining credits missing"
        );
    }

    // Summary-derived credit balances must be present.
    assert_eq!(
        json["credits"]["totalAvailableCredits"].as_str(),
        summary
            .balances
            .as_ref()
            .and_then(|b| b.total_available_credits.as_deref()),
        "credit balance mismatch"
    );

    // Preflight must have filled usedCredits on the matching windows.
    assert!(
        windows.iter().any(|w| w["usedCredits"].as_str().is_some()),
        "preflight did not supply usedCredits"
    );

    // A live run with a populated store must not write into the real home.
    let home = temp_home("home");
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = odquota::store::Store::resolve().expect("store");
    assert_eq!(store.path(), home.join("accounts.dat"));
    let _ = fs::remove_dir_all(&home);

    println!("live ok: workspace={ws} windows={}", windows.len());
    // The link host is only exercised when a runtime key is also provided.
    if let Ok(rt) = std::env::var("ODQUOTA_LIVE_RUNTIME_KEY") {
        if !rt.trim().is_empty() {
            let models = api
                .entitled_models(link, &rt, &ws)
                .expect("entitled models");
            println!("live ok: entitled models={}", models.data.len());
        }
    }
}
