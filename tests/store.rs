//! Store tests: DPAPI round-trip, atomic replace, and the guarantee that the
//! credential bytes never appear in plaintext on disk.
//!
//! Every test writes into a temporary directory and points `ODQUOTA_HOME` at it
//! so the real `%APPDATA%` store is never touched.

use std::fs;
use std::path::PathBuf;

use odquota::store::{AccountEntry, Store, StoreDocument};

/// Tests must exercise the encrypted path on every platform. Windows uses
/// DPAPI and ignores this; Unix derives a key from it.
fn use_test_passphrase() {
    std::env::set_var("ODQUOTA_PASSPHRASE", "odquota-test-passphrase");
}

fn temp_home(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("odquota-test-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create temp dir");
    base
}

fn sample_entry(label: &str, control: &str) -> AccountEntry {
    AccountEntry {
        label: label.to_string(),
        control_key: control.to_string(),
        runtime_key: Some("sk-amr-test-runtime-value".to_string()),
        api_url: "https://amr-api.open-design.ai".to_string(),
        link_url: "https://amr-link.open-design.ai".to_string(),
        workspace_id: Some("ws-test".to_string()),
        api_url_source: "manual".to_string(),
        added_at: "2026-11-08T00:00:00Z".to_string(),
        source: "test".to_string(),
    }
}

#[test]
fn save_and_load_roundtrip() {
    let home = temp_home("roundtrip");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");

    let mut doc = StoreDocument::default();
    doc.accounts
        .push(sample_entry("main", "vela_ctrl_TESTKEY0001"));
    store.save(&doc).expect("save");

    let loaded = store.load().expect("load");
    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.accounts.len(), 1);
    assert_eq!(loaded.accounts[0].label, "main");
    assert_eq!(loaded.accounts[0].control_key, "vela_ctrl_TESTKEY0001");
    assert_eq!(
        loaded.accounts[0].runtime_key.as_deref(),
        Some("sk-amr-test-runtime-value")
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn missing_store_yields_empty_document() {
    let home = temp_home("empty");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");
    let doc = store.load().expect("missing file is not an error");
    assert_eq!(doc.accounts.len(), 0);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn credentials_never_appear_in_plaintext_on_disk() {
    let home = temp_home("plaintext");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");

    let control = "vela_ctrl_SECRETPROBE0001";
    let runtime = "sk-amr-SECRETPROBE0002";
    let mut doc = StoreDocument::default();
    let mut entry = sample_entry("probe", control);
    entry.runtime_key = Some(runtime.to_string());
    doc.accounts.push(entry);
    store.save(&doc).expect("save");

    let bytes = fs::read(store.path()).expect("read store file");
    let text = String::from_utf8_lossy(&bytes);

    assert!(
        !text.contains(control),
        "controlKey found in plaintext on disk: {}",
        store.path().display()
    );
    assert!(
        !text.contains(runtime),
        "runtimeKey found in plaintext on disk: {}",
        store.path().display()
    );
    assert!(
        !text.contains("SECRETPROBE"),
        "secret marker found in plaintext on disk"
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn save_replaces_atomically_without_leftover_temp_files() {
    let home = temp_home("atomic");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");

    let mut doc = StoreDocument::default();
    doc.accounts.push(sample_entry("first", "vela_ctrl_FIRST"));
    store.save(&doc).expect("first save");

    doc.accounts
        .push(sample_entry("second", "vela_ctrl_SECOND"));
    store.save(&doc).expect("second save");

    let leftovers: Vec<_> = fs::read_dir(&home)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with(".accounts."))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files were left behind: {:?}",
        leftovers.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );

    let loaded = store.load().expect("load after replace");
    assert_eq!(loaded.accounts.len(), 2);

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn corrupted_store_is_reported_not_silently_ignored() {
    let home = temp_home("corrupt");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");
    fs::write(store.path(), b"this is not a store").expect("write garbage");

    let err = store.load().expect_err("must report corruption");
    assert_eq!(err.kind(), odquota::error::Kind::Store);
    assert!(
        err.message().contains("復号") || err.message().contains("壊れ"),
        "unexpected message: {}",
        err.message()
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn version_mismatch_is_rejected() {
    let home = temp_home("version");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");

    // Write a document that claims a future version; loading must refuse it
    // rather than silently parsing fields it does not understand.
    let doc = StoreDocument {
        version: 99,
        accounts: vec![sample_entry("v", "vela_ctrl_V")],
    };
    store.save(&doc).expect("save");

    let err = store.load().expect_err("future version must be rejected");
    assert_eq!(err.kind(), odquota::error::Kind::Store);
    assert!(
        err.message().contains("version 99"),
        "unexpected message: {}",
        err.message()
    );

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn empty_file_is_treated_as_first_run() {
    let home = temp_home("zero");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");
    fs::write(store.path(), b"").expect("write empty file");

    let doc = store.load().expect("empty file is not corruption");
    assert_eq!(doc.accounts.len(), 0);

    let _ = fs::remove_dir_all(&home);
}

#[test]
fn store_lives_under_odquota_home_not_appdata() {
    let home = temp_home("home");
    use_test_passphrase();
    std::env::set_var("ODQUOTA_HOME", &home);
    let store = Store::resolve().expect("resolve store");
    assert_eq!(store.path(), home.join("accounts.dat"));
    let _ = fs::remove_dir_all(&home);
}
