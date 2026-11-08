//! End-to-end CLI tests: run the built binary against a mock API.
//!
//! `ODQUOTA_HOME` is set per test so the real store is never touched. The
//! binary path comes from Cargo's `CARGO_BIN_EXE_odquota`, which always exists
//! for integration tests of a crate that produces a binary.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

use serde_json::Value;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_odquota")
}

fn temp_home(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("odquota-cli-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create temp home");
    base
}

/// Serve the four endpoints the quota command needs, each once, in order.
/// Returns the base URL.
fn serve_quota_flow() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");

    let workspaces = r#"{"items":[{"memberCount":1,"isDefault":true,"workspaceId":"ws-mock","workspaceName":"Mock WS","workspaceIconKey":"person","workspaceType":"personal","workspaceMemberId":"wm-mock","role":"owner","memberStatus":"active","lifecycleState":"active"}]}"#;
    let me = r#"{"id":"user-mock","email":"mock@example.test","status":"active","plan":"free","balanceUsd":"1.5000"}"#;
    let snapshot = r#"{"schemaVersion":1,"billingScopeVersion":2,"billingMode":"shared","workspaceId":"ws-mock","workspaceMemberId":"wm-mock","billing":{"billingState":"active","planId":"go"},"plan":{"tier":"go","interval":"month","status":"active","cancelAtPeriodEnd":false},"memberCount":1,"wallet":{"balanceUsd":"1.5000","expiresAt":"2026-11-09T13:04:25.639Z"},"walletBreakdown":[{"kind":"signup_gift","amountUsd":"0.5000","expiresAt":"2026-11-09T13:04:25.639Z","restriction":{"allowedModels":["public_model_deepseek_v4_1_flash"]}}],"designPlanWindows":[{"kind":"5h","windowStart":"2026-11-08T06:51:55.310Z","resetsAt":"2026-11-08T11:51:55.310Z","remainingCredits":"47320","limitCredits":"50000"},{"kind":"7d","windowStart":"2026-11-07T10:51:55.310Z","resetsAt":"2026-11-14T10:51:55.310Z","remainingCredits":"116039","limitCredits":"150000"}],"permissions":{"canUpgrade":true,"canRecharge":true,"canConfigureAutoRecharge":true,"canManageSubscription":true}}"#;
    let summary = r#"{"balanceUsd":"1.5000","creditsPerUsd":10000,"balances":{"subscriptionCredits":"10000","rechargeCredits":"5000","totalAvailableCredits":"15000"},"subscriptionCreditsUsd":"1.0000","rechargeCreditsUsd":"0.5000","totalAvailableCreditsUsd":"1.5000","membershipTier":"go","billingInterval":"monthly","subscriptionStatus":"active","totalRechargedUsd":"0.5000","totalConsumedUsd":"0.0000","todayConsumedUsd":"0.0000","rechargeCount":1,"usageCount":9818,"updatedAt":"2026-11-08T09:02:28.899Z"}"#;

    let auto_recharge = r#"{"enabled":false,"lowerThresholdUsd":"10.00","topupAmountUsd":"50.00","monthlyLimitUsd":"200.00","monthlyLimitUnlimited":false,"paymentMethod":{"status":"ok"},"month":{"succeededUsd":"0.0000","inFlightUsd":"0.0000","periodStart":"2026-11-01T00:00:00.000Z","periodEnd":"2026-12-01T00:00:00.000Z"},"lastFailure":null,"inFlight":null}"#;

    let bodies: Vec<String> = vec![
        workspaces.to_string(),
        me.to_string(),
        snapshot.to_string(),
        auto_recharge.to_string(),
        summary.to_string(),
    ];

    let handle = thread::spawn(move || {
        for body in bodies {
            match listener.accept() {
                Ok((mut stream, _peer)) => {
                    let mut buf = [0u8; 8192];
                    let _ = stream.read(&mut buf);
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                }
                Err(_) => break,
            }
        }
    });

    (format!("http://{}", addr), handle)
}

#[test]
fn version_flag_works() {
    let out = Command::new(bin())
        .arg("--version")
        .output()
        .expect("run --version");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains(env!("CARGO_PKG_VERSION")),
        "version output missing version: {text}"
    );
}

#[test]
fn account_list_on_empty_store_is_valid_json() {
    let home = temp_home("list");

    let out = Command::new(bin())
        .args(["account", "list"])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("run account list");

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["accounts"].as_array().unwrap().len(), 0);
    // Windows always encrypts (DPAPI). Unix encrypts only when a passphrase is
    // supplied, so assert the reported protection instead of a fixed boolean.
    let protection = json["storage"]["protection"].as_str().unwrap_or("");
    assert!(
        matches!(protection, "dpapi" | "passphrase" | "plain"),
        "unexpected protection: {protection}"
    );
    assert_eq!(
        json["storage"]["encrypted"].as_bool().unwrap(),
        protection != "plain"
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn quota_with_empty_store_exits_with_empty_code() {
    let home = temp_home("emptyquota");

    let out = Command::new(bin())
        .arg("quota")
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("run quota");

    assert_eq!(
        out.status.code(),
        Some(3),
        "expected EXIT_EMPTY, got {:?} (stderr: {})",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("保存済みアカウントがありません"),
        "stderr did not explain the empty store"
    );
    assert!(out.stdout.is_empty(), "stdout should be empty on exit 3");

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn add_rejects_reading_key_from_argv() {
    // No --stdin and no --control-key-env must be a usage error, not a prompt.
    let home = temp_home("addusage");

    let out = Command::new(bin())
        .args(["account", "add", "--label", "x"])
        .env("ODQUOTA_HOME", &home)
        .stdin(Stdio::null())
        .output()
        .expect("run account add");

    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("鍵を引数には取りません"));

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn quota_end_to_end_against_mock_api() {
    let home = temp_home("e2e");
    let (base, handle) = serve_quota_flow();

    // Register the account non-interactively via an env var name, then query.
    let add = Command::new(bin())
        .args([
            "account",
            "add",
            "--label",
            "mock",
            "--control-key-env",
            "ODQUOTA_TEST_KEY",
            "--api-url",
            &base,
            "--link-url",
            &base,
        ])
        .env("ODQUOTA_HOME", &home)
        .env("ODQUOTA_TEST_KEY", "tp_test_key_value")
        .output()
        .expect("account add");
    assert!(
        add.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    let quota = Command::new(bin())
        .args(["quota", "--pretty"])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("quota");
    handle.join().expect("server joined");

    assert_eq!(
        quota.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&quota.stderr)
    );

    let json: Value = serde_json::from_slice(&quota.stdout).expect("valid JSON");
    assert_eq!(json["schemaVersion"], 1);
    let acc = &json["accounts"][0];
    assert_eq!(acc["label"], "mock");
    assert!(acc["ok"].as_bool().unwrap());
    assert!(acc["complete"].as_bool().unwrap());
    assert_eq!(acc["identity"]["userId"], "user-mock");
    assert_eq!(acc["workspace"]["workspaceId"], "ws-mock");
    assert_eq!(acc["workspace"]["source"], "resolved");
    assert_eq!(acc["quota"]["plan"]["tier"], "go");
    assert_eq!(acc["quota"]["windows"][0]["kind"], "5h");
    assert_eq!(acc["quota"]["windows"][0]["remainingPercent"], 94.64);
    assert_eq!(acc["quota"]["windows"][1]["kind"], "7d");
    assert_eq!(acc["quota"]["credits"]["totalAvailableCredits"], "15000");
    assert_eq!(acc["quota"]["wallet"]["balanceUsd"], "1.5000");
    assert_eq!(acc["errors"].as_array().unwrap().len(), 0);

    // The stored file must not contain the key in plaintext.
    let store = home.join("accounts.dat");
    let bytes = fs::read(&store).expect("read store");
    let text = String::from_utf8_lossy(&bytes);
    let protection = json["storage"]["protection"].as_str().unwrap_or("");
    if protection != "plain" {
        assert!(
            !text.contains("tp_test_key_value"),
            "control key found in plaintext while protection={protection}"
        );
    }

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn partial_failure_reports_ok_false_and_exit_1() {
    let home = temp_home("partial");

    let add = Command::new(bin())
        .args([
            "account",
            "add",
            "--label",
            "good",
            "--control-key-env",
            "ODQUOTA_GOOD_KEY",
            "--api-url",
            "http://127.0.0.1:9",
        ])
        .env("ODQUOTA_HOME", &home)
        .env("ODQUOTA_GOOD_KEY", "ctrl_good")
        .output()
        .expect("add good");

    // A second account whose api-url is a closed port fails every stage.
    let add2 = Command::new(bin())
        .args([
            "account",
            "add",
            "--label",
            "bad",
            "--control-key-env",
            "ODQUOTA_BAD_KEY",
            "--api-url",
            "http://127.0.0.1:9",
        ])
        .env("ODQUOTA_HOME", &home)
        .env("ODQUOTA_BAD_KEY", "ctrl_bad")
        .output()
        .expect("add bad");

    assert!(add.status.success());
    assert!(add2.status.success());

    let quota = Command::new(bin())
        .args(["quota"])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("quota");

    assert_eq!(quota.status.code(), Some(1), "expected EXIT_PARTIAL");
    let json: Value = serde_json::from_slice(&quota.stdout).expect("valid JSON");
    assert_eq!(json["accounts"].as_array().unwrap().len(), 2);
    for acc in json["accounts"].as_array().unwrap() {
        assert!(
            !acc["ok"].as_bool().unwrap(),
            "unreachable API must not report ok"
        );
        assert!(
            acc["quota"].is_null(),
            "failed account must not carry a quota object"
        );
        let errors = acc["errors"].as_array().unwrap();
        assert!(!errors.is_empty(), "failed account has no error entries");
        assert!(
            errors[0]["message"]
                .as_str()
                .unwrap()
                .contains("接続できません"),
            "error message did not explain the connection failure: {}",
            errors[0]
        );
    }

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn import_amr_rejects_missing_control_key() {
    let home = temp_home("amr");
    let cfg = home.join("config.json");
    fs::write(
        &cfg,
        r#"{"profiles":{"prod":{"apiUrl":"https://example.test","linkUrl":"https://example.test"}}}"#,
    )
    .expect("write config");

    let out = Command::new(bin())
        .args(["account", "import-amr", "--config", cfg.to_str().unwrap()])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("import");

    assert_eq!(out.status.code(), Some(2), "missing key is a usage error");
    assert!(String::from_utf8_lossy(&out.stderr).contains("controlKey がありません"));

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn import_amr_stores_profile_and_refuses_duplicate_without_force() {
    let home = temp_home("amrdup");
    let cfg = home.join("config.json");
    fs::write(
        &cfg,
        r#"{"profiles":{"prod":{"controlKey":"vela_ctrl_IMPORTED","runtimeKey":"sk-amr-IMPORTED","apiUrl":"https://example.test","linkUrl":"https://example.test"}}}"#,
    )
    .expect("write config");

    let first = Command::new(bin())
        .args(["account", "import-amr", "--config", cfg.to_str().unwrap()])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("first import");
    assert!(first.status.success());

    let second = Command::new(bin())
        .args(["account", "import-amr", "--config", cfg.to_str().unwrap()])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("second import");
    assert_eq!(second.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&second.stderr).contains("--force"));

    let forced = Command::new(bin())
        .args([
            "account",
            "import-amr",
            "--config",
            cfg.to_str().unwrap(),
            "--force",
        ])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("forced import");
    assert!(forced.status.success());

    let list = Command::new(bin())
        .args(["account", "list"])
        .env("ODQUOTA_HOME", &home)
        .output()
        .expect("list");
    let json: Value = serde_json::from_slice(&list.stdout).expect("valid JSON");
    assert_eq!(json["accounts"].as_array().unwrap().len(), 1);
    assert_eq!(json["accounts"][0]["label"], "prod");
    assert!(json["accounts"][0]["hasRuntimeKey"].as_bool().unwrap());
    // The list must never expose the key itself.
    assert!(!list.stdout.windows(8).any(|w| w == b"vela_ctrl"));

    let _ = fs::remove_dir_all(&home);
}
