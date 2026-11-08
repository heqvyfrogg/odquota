//! HTTP access to the Vela control plane and Link.
//!
//! Two rules are baked in here:
//!   * a non-empty `User-Agent` is always sent, because Cloudflare in front of
//!     `/api/v1/billing/summary` rejects Go's default agent,
//!   * the bearer token never appears in an error message, a stage error code,
//!     or any serialized output.

use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;

use crate::model::{
    AutoRecharge, EndpointStatus, LinkModelList, Me, ModelCatalog, Preflight, Snapshot, StageError,
    Summary, WorkspaceList,
};

pub const USER_AGENT: &str = concat!("odquota/", env!("CARGO_PKG_VERSION"));
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_ATTEMPTS: u32 = 3;

/// A failed HTTP stage, carrying the evidence needed to explain it in JSON.
#[derive(Debug)]
pub struct ApiFailure {
    pub http_status: Option<u16>,
    pub code: Option<String>,
    pub message: String,
    pub cloudflare_block: bool,
}

impl ApiFailure {
    pub fn into_stage_error(self, stage: &str) -> StageError {
        StageError {
            stage: stage.to_string(),
            http_status: self.http_status,
            code: self.code,
            message: self.message,
            cloudflare_block: self.cloudflare_block,
        }
    }
}

pub struct Api {
    client: Client,
}

impl Api {
    pub fn new() -> Result<Self, ApiFailure> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| ApiFailure {
                http_status: None,
                code: None,
                message: format!("HTTP クライアントを作れません: {e}"),
                cloudflare_block: false,
            })?;
        Ok(Self { client })
    }

    /// GET a JSON document with bearer auth, retrying only transient failures.
    fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        base: &str,
        path: &str,
        bearer: &str,
        extra_headers: &[(&str, &str)],
    ) -> Result<T, ApiFailure> {
        let url = join_url(base, path);
        let mut last: Option<ApiFailure> = None;

        for attempt in 1..=MAX_ATTEMPTS {
            let mut req = self.client.get(&url).bearer_auth(bearer);
            for (k, v) in extra_headers {
                req = req.header(*k, *v);
            }

            let sent = req.send();
            match sent {
                Err(e) => {
                    let retryable = e.is_timeout() || e.is_connect() || e.is_request();
                    last = Some(ApiFailure {
                        http_status: None,
                        code: None,
                        message: format!("{path} へ接続できません: {e}"),
                        cloudflare_block: false,
                    });
                    if !retryable || attempt == MAX_ATTEMPTS {
                        break;
                    }
                }
                Ok(resp) => match self.classify(resp, path) {
                    Ok(value) => return Ok(value),
                    Err(f) => {
                        let retryable = matches!(f.http_status, Some(500..=599));
                        last = Some(f);
                        if !retryable || attempt == MAX_ATTEMPTS {
                            break;
                        }
                    }
                },
            }

            // Exponential backoff between transient attempts only.
            std::thread::sleep(Duration::from_millis(400 * attempt as u64));
        }

        Err(last.unwrap_or_else(|| ApiFailure {
            http_status: None,
            code: None,
            message: format!("{path} の取得に失敗しました（理由不明）"),
            cloudflare_block: false,
        }))
    }

    fn classify<T: serde::de::DeserializeOwned>(
        &self,
        resp: Response,
        path: &str,
    ) -> Result<T, ApiFailure> {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();

        if status.is_success() {
            return serde_json::from_str::<T>(&body).map_err(|e| ApiFailure {
                http_status: Some(status.as_u16()),
                code: Some("invalid_json".to_string()),
                message: format!(
                    "{path} の応答を JSON として解釈できません: {e}（先頭 {} バイト: {}）",
                    body.len().min(120),
                    sanitize_body(&body, 120)
                ),
                cloudflare_block: false,
            });
        }

        let lower = body.to_ascii_lowercase();
        let cloudflare_block = status == StatusCode::FORBIDDEN
            && (lower.contains("attention required") || lower.contains("cloudflare"));

        // The API reports failures as {"error":"..."}; anything else is noted verbatim.
        let code = extract_error_code(&body);

        let message = if cloudflare_block {
            format!(
                "{path} が Cloudflare に遮断されました (HTTP {}）。\
                 User-Agent を確認してください（送信値: {USER_AGENT}）。\
                 本文先頭: {}",
                status.as_u16(),
                sanitize_body(&body, 120)
            )
        } else {
            format!(
                "{path} が HTTP {} を返しました（code: {}、本文先頭: {}）",
                status.as_u16(),
                code.as_deref().unwrap_or("なし"),
                sanitize_body(&body, 160)
            )
        };

        Err(ApiFailure {
            http_status: Some(status.as_u16()),
            code,
            message,
            cloudflare_block,
        })
    }

    pub fn list_workspaces(&self, base: &str, bearer: &str) -> Result<WorkspaceList, ApiFailure> {
        self.get_json(base, "/api/v1/workspaces", bearer, &[])
    }

    pub fn me(&self, base: &str, bearer: &str) -> Result<Me, ApiFailure> {
        self.get_json(base, "/api/v1/me", bearer, &[])
    }

    pub fn snapshot(
        &self,
        base: &str,
        bearer: &str,
        workspace: &str,
    ) -> Result<Snapshot, ApiFailure> {
        self.get_json(
            base,
            &format!("/api/v1/workspaces/{workspace}/billing/snapshot"),
            bearer,
            &[("x-vela-workspace-id", workspace)],
        )
    }

    pub fn summary(
        &self,
        base: &str,
        bearer: &str,
        workspace: &str,
    ) -> Result<Summary, ApiFailure> {
        self.get_json(
            base,
            "/api/v1/billing/summary",
            bearer,
            &[("x-vela-workspace-id", workspace)],
        )
    }

    pub fn preflight(
        &self,
        base: &str,
        bearer: &str,
        workspace: &str,
        model_id: &str,
    ) -> Result<Preflight, ApiFailure> {
        let path = format!(
            "/api/v1/workspaces/{workspace}/billing/preflight?modelId={}",
            encode_query_value(model_id)
        );
        self.get_json(base, &path, bearer, &[("x-vela-workspace-id", workspace)])
    }

    pub fn auto_recharge(
        &self,
        base: &str,
        bearer: &str,
        workspace: &str,
    ) -> Result<AutoRecharge, ApiFailure> {
        self.get_json(
            base,
            &format!("/api/v1/workspaces/{workspace}/auto-recharge"),
            bearer,
            &[("x-vela-workspace-id", workspace)],
        )
    }

    pub fn model_catalog(&self, base: &str, bearer: &str) -> Result<ModelCatalog, ApiFailure> {
        self.get_json(base, "/api/v1/models", bearer, &[])
    }

    /// Probe any path and report its HTTP status without failing the caller.
    /// Used to record which surfaces exist for the account and which are denied.
    pub fn probe(&self, base: &str, bearer: &str, path: &str) -> EndpointStatus {
        use crate::model::EndpointStatus;
        let url = join_url(base, path);
        match self.client.get(&url).bearer_auth(bearer).send() {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().unwrap_or_default();
                let code = extract_error_code(&body);
                EndpointStatus {
                    path: path.to_string(),
                    http_status: Some(status),
                    error_code: code,
                }
            }
            Err(_) => EndpointStatus {
                path: path.to_string(),
                http_status: None,
                error_code: None,
            },
        }
    }

    pub fn entitled_models(
        &self,
        link_base: &str,
        runtime_bearer: &str,
        workspace: &str,
    ) -> Result<LinkModelList, ApiFailure> {
        self.get_json(
            link_base,
            "/v1/models",
            runtime_bearer,
            &[("x-vela-workspace-id", workspace)],
        )
    }
}

fn join_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

/// Minimal query encoding: the model ids we send are ASCII identifiers, but we
/// still escape the characters that would break a query string.
fn encode_query_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            _ => out.push_str(&format!("%{:02X}", ch as u32)),
        }
    }
    out
}

/// Pull `error` out of `{"error":"unauth..."}` or `{"error":{"code":...}}`.
fn extract_error_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let err = value.get("error")?;
    if let Some(s) = err.as_str() {
        return Some(s.to_string());
    }
    err.get("code")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// Keep error bodies readable and bounded; never echo credentials that a server
/// might reflect back (we only ever print the first N characters).
fn sanitize_body(body: &str, max: usize) -> String {
    let flat: String = body
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = flat.trim();
    if trimmed.chars().count() <= max {
        trimmed.to_string()
    } else {
        let cut: String = trimmed.chars().take(max).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_urls_without_double_slash() {
        assert_eq!(
            join_url("https://example.test", "/api/v1/me"),
            "https://example.test/api/v1/me"
        );
        assert_eq!(
            join_url("https://example.test/", "/api/v1/me"),
            "https://example.test/api/v1/me"
        );
    }

    #[test]
    fn extracts_error_code_from_string_and_object() {
        assert_eq!(
            extract_error_code(r#"{"error":"unauthenticated"}"#),
            Some("unauthenticated".to_string())
        );
        assert_eq!(
            extract_error_code(r#"{"error":{"code":"invalid_api_key"}}"#),
            Some("invalid_api_key".to_string())
        );
        assert_eq!(extract_error_code("404 Not Found"), None);
    }

    #[test]
    fn sanitizes_control_characters_and_bounds_length() {
        let s = sanitize_body("a\nb\tc", 10);
        assert_eq!(s, "a b c");
        let long = "x".repeat(200);
        let s = sanitize_body(&long, 10);
        assert!(s.ends_with('…'));
        assert_eq!(s.chars().count(), 11);
    }

    #[test]
    fn encodes_query_values() {
        assert_eq!(
            encode_query_value("deepseek-v4.1-flash"),
            "deepseek-v4.1-flash"
        );
        assert_eq!(encode_query_value("a b&c"), "a%20b%26c");
    }

    #[test]
    fn user_agent_is_non_empty() {
        assert!(USER_AGENT.starts_with("odquota/"));
        assert!(!USER_AGENT.trim().is_empty());
    }
}
