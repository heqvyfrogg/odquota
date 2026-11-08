//! Shared JSON writer for every subcommand.

use std::io::{self, Write};

use crate::error::AppError;

pub fn print_json<T: serde::Serialize>(value: &T, pretty: bool) -> Result<(), AppError> {
    let text = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|e| AppError::store(format!("JSON を作れません: {e}")))?;

    let mut out = io::stdout();
    out.write_all(text.as_bytes())
        .and_then(|_| out.write_all(b"\n"))
        .and_then(|_| out.flush())
        .map_err(|e| AppError::store(format!("標準出力へ書けません: {e}")))
}
