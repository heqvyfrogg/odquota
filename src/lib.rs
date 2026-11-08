//! odquota library: credential storage, Vela billing API access, JSON reporting.
//!
//! The binary in `main.rs` is a thin wrapper; everything testable lives here so
//! integration tests can exercise the same code paths the CLI runs.

pub mod amr;
pub mod api;
pub mod app;
pub mod cli;
#[cfg(not(windows))]
pub mod crypto;
pub mod error;
pub mod model;
pub mod output;
pub mod report;
pub mod store;
