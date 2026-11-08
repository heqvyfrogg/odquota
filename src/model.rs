//! API response shapes and the CLI output document.
//!
//! Credit amounts come back from the API as decimal strings; they are kept as
//! strings end to end so no precision is lost and no value is invented when a
//! field is missing.

use serde::{Deserialize, Serialize};

pub const OUTPUT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------- API: base

#[derive(Debug, Deserialize)]
pub struct WorkspaceList {
    pub items: Vec<WorkspaceItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceItem {
    pub workspace_id: String,
    pub workspace_member_id: String,
    #[serde(default)]
    pub workspace_icon_key: Option<String>,
    #[serde(default)]
    pub workspace_name: Option<String>,
    #[serde(default)]
    pub workspace_type: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub is_default: Option<bool>,
    #[serde(default)]
    pub member_count: Option<u64>,
    #[serde(default)]
    pub member_status: Option<String>,
    #[serde(default)]
    pub lifecycle_state: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: String,
    #[serde(default)]
    pub auth_user_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub balance_usd: Option<String>,
}

// ---------------------------------------------------------------- API: quota

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub billing_scope_version: Option<u32>,
    #[serde(default)]
    pub billing_mode: Option<String>,
    pub workspace_id: String,
    pub workspace_member_id: String,
    #[serde(default)]
    pub billing: Option<BillingRef>,
    #[serde(default)]
    pub plan: Option<SnapshotPlan>,
    #[serde(default)]
    pub member_count: Option<u64>,
    #[serde(default)]
    pub wallet: Option<Wallet>,
    #[serde(default)]
    pub wallet_breakdown: Option<Vec<WalletGrant>>,
    #[serde(default)]
    pub design_plan_windows: Option<Vec<SnapshotWindow>>,
    #[serde(default)]
    pub permissions: Option<Permissions>,
    #[serde(default)]
    pub owner: Option<Owner>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    #[serde(default)]
    pub workspace_member_id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingRef {
    #[serde(default)]
    pub billing_state: Option<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPlan {
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub interval: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub current_period_start: Option<String>,
    #[serde(default)]
    pub current_period_end: Option<String>,
    #[serde(default)]
    pub cancel_at_period_end: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    #[serde(default)]
    pub balance_usd: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletGrant {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub amount_usd: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub restriction: Option<GrantRestriction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantRestriction {
    #[serde(default)]
    pub allowed_models: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotWindow {
    pub kind: String,
    #[serde(default)]
    pub window_start: Option<String>,
    #[serde(default)]
    pub resets_at: Option<String>,
    #[serde(default)]
    pub remaining_credits: Option<String>,
    #[serde(default)]
    pub limit_credits: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct Permissions {
    #[serde(default)]
    pub can_upgrade: Option<bool>,
    #[serde(default)]
    pub can_recharge: Option<bool>,
    #[serde(default)]
    pub can_configure_auto_recharge: Option<bool>,
    #[serde(default)]
    pub can_manage_subscription: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    #[serde(default)]
    pub balance_usd: Option<String>,
    #[serde(default)]
    pub credits_per_usd: Option<u64>,
    #[serde(default)]
    pub balances: Option<SummaryBalances>,
    #[serde(default)]
    pub subscription_credits_usd: Option<String>,
    #[serde(default)]
    pub recharge_credits_usd: Option<String>,
    #[serde(default)]
    pub total_available_credits_usd: Option<String>,
    #[serde(default)]
    pub membership_tier: Option<String>,
    #[serde(default)]
    pub billing_interval: Option<String>,
    #[serde(default)]
    pub subscription_status: Option<String>,
    #[serde(default)]
    pub subscription_entitlement_status: Option<String>,
    #[serde(default)]
    pub subscription_cancel_at_period_end: Option<bool>,
    #[serde(default)]
    pub subscription_current_period_start: Option<String>,
    #[serde(default)]
    pub subscription_current_period_end: Option<String>,
    #[serde(default)]
    pub today_consumed_usd: Option<String>,
    #[serde(default)]
    pub total_consumed_usd: Option<String>,
    #[serde(default)]
    pub total_recharged_usd: Option<String>,
    #[serde(default)]
    pub recharge_count: Option<u64>,
    #[serde(default)]
    pub usage_count: Option<u64>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryBalances {
    #[serde(default)]
    pub subscription_credits: Option<String>,
    #[serde(default)]
    pub recharge_credits: Option<String>,
    #[serde(default)]
    pub total_available_credits: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub workspace_member_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub balance_usd: Option<String>,
    #[serde(default)]
    pub coding_plan: Option<CodingPlan>,
    #[serde(default)]
    pub model_covered: Option<bool>,
    #[serde(default)]
    pub funding: Option<String>,
    #[serde(default)]
    pub funding_check: Option<String>,
    #[serde(default)]
    pub billing_mode: Option<String>,
    #[serde(default)]
    pub task_check: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodingPlan {
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub eligible: Option<bool>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub windows: Option<Vec<CodingWindow>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodingWindow {
    #[serde(default)]
    pub policy_id: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<u64>,
    #[serde(default)]
    pub used_credits: Option<String>,
    #[serde(default)]
    pub limit_credits: Option<String>,
    #[serde(default)]
    pub remaining_credits: Option<String>,
    #[serde(default)]
    pub window_start: Option<String>,
    #[serde(default)]
    pub resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkModelList {
    pub data: Vec<LinkModel>,
}

// Link returns snake_case keys (`owned_by`, `context_budget`, `output_budget`),
// unlike the control plane, so these structs deliberately have no rename_all.
#[derive(Debug, Deserialize)]
pub struct LinkModel {
    pub id: String,
    #[serde(default)]
    pub metadata: Option<LinkMetadata>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LinkMetadata {
    #[serde(default)]
    pub cost: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub context_budget: Option<u64>,
    #[serde(default)]
    pub output_budget: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ModelCatalog {
    pub data: Vec<CatalogModel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub metadata: Option<CatalogMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct CatalogMetadata {
    #[serde(default)]
    pub cost: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoRecharge {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub lower_threshold_usd: Option<String>,
    #[serde(default)]
    pub topup_amount_usd: Option<String>,
    #[serde(default)]
    pub monthly_limit_usd: Option<String>,
    #[serde(default)]
    pub monthly_limit_unlimited: Option<bool>,
    #[serde(default)]
    pub payment_method: Option<PaymentMethod>,
    #[serde(default)]
    pub month: Option<RechargeMonth>,
    #[serde(default)]
    pub last_failure: Option<serde_json::Value>,
    #[serde(default)]
    pub in_flight: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentMethod {
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeMonth {
    #[serde(default)]
    pub succeeded_usd: Option<String>,
    #[serde(default)]
    pub in_flight_usd: Option<String>,
    #[serde(default)]
    pub period_start: Option<String>,
    #[serde(default)]
    pub period_end: Option<String>,
}

// ------------------------------------------------------------- CLI output

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u32,
    pub generated_at: String,
    pub storage: StorageInfo,
    pub accounts: Vec<AccountReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub path: String,
    pub encrypted: bool,
    /// "dpapi" | "passphrase" | "plain". Exposed so a caller never has to
    /// guess from the boolean alone how the file is protected.
    pub protection: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountReport {
    pub label: String,
    /// True when the base quota was obtained. Answers "did the read work".
    pub ok: bool,
    /// True when `ok` and every stage the caller asked for succeeded. A
    /// missing optional stage (say, `--entitled` without a runtimeKey) leaves
    /// `ok` true here and `complete` false.
    pub complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<QuotaOut>,
    /// Which read-only endpoints answered, and how. Recorded even on success so
    /// the report shows what exists for this account rather than implying that
    /// everything was queried and came back empty.
    pub surfaces: Vec<EndpointStatus>,
    /// Only present with --catalog; the full public model list, not just this
    /// account's entitlements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog: Option<Vec<ModelOut>>,
    pub errors: Vec<StageError>,
}

impl AccountReport {
    pub fn failed(label: &str, errors: Vec<StageError>) -> Self {
        Self {
            label: label.to_string(),
            ok: false,
            complete: false,
            fetched_at: None,
            identity: None,
            workspace: None,
            quota: None,
            surfaces: Vec::new(),
            catalog: None,
            errors,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_usd: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOut {
    pub workspace_id: String,
    pub workspace_member_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub workspace_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<String>,
    pub source: WorkspaceSource,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceSource {
    Stored,
    Resolved,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing: Option<BillingOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits: Option<CreditsOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<WalletOut>,
    pub windows: Vec<WindowOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption: Option<ConsumptionOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionsOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight: Option<PreflightOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitled_models: Option<Vec<EntitledModelOut>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_recharge: Option<AutoRechargeOut>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_mode: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at_period_end: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditsOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_credits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recharge_credits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_available_credits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits_per_usd: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_credits_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recharge_credits_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_available_credits_usd: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub breakdown: Vec<WalletGrantOut>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletGrantOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowOut {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_credits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_credits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_credits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percent: Option<f64>,
    pub exhausted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_in_seconds: Option<i64>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsumptionOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub today_consumed_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_consumed_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_recharged_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recharge_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_upgrade: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_recharge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_configure_auto_recharge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_manage_subscription: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_covered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoRechargeOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_threshold_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topup_amount_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_limit_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_limit_unlimited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_succeeded_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_in_flight_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_last_failure: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStatus {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOut {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitledModelOut {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_budget: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_budget: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageError {
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "is_false")]
    pub cloudflare_block: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl StageError {
    pub fn new(stage: &str, message: impl Into<String>) -> Self {
        Self {
            stage: stage.to_string(),
            http_status: None,
            code: None,
            message: message.into(),
            cloudflare_block: false,
        }
    }
}

// --------------------------------------------------------- CLI: account list

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountListOutput {
    pub schema_version: u32,
    pub storage: StorageInfo,
    pub accounts: Vec<AccountListItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountListItem {
    pub label: String,
    pub api_url: String,
    pub link_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub has_runtime_key: bool,
    pub added_at: String,
    pub source: String,
}
