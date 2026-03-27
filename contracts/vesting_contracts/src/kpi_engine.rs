// Issue #145 / #92 — Oracle-Verified KPI Vesting Triggers
// KPI Engine: stores config, verifies oracle values, flips idempotent flag.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol, Vec};
use crate::oracle::ComparisonOperator;

// ── Storage key symbols ───────────────────────────────────────────────────

pub fn kpi_config_key(vault_id: u64) -> (Symbol, u64) {
    (symbol_short!("KpiCfg"), vault_id)
}

pub fn kpi_met_key(vault_id: u64) -> (Symbol, u64) {
    (symbol_short!("KpiMet"), vault_id)
}

pub fn kpi_log_key(vault_id: u64) -> (Symbol, u64) {
    (symbol_short!("KpiLog"), vault_id)
}

// ── Types ─────────────────────────────────────────────────────────────────

/// On-chain config for one vault's KPI gate.
#[contracttype]
#[derive(Clone, Debug)]
pub struct KpiOracleConfig {
    /// Soroban contract that exposes `query_kpi(metric_id: Symbol) -> i128`
    pub oracle_contract: Address,
    /// Metric identifier forwarded to the oracle (≤ 10 chars).
    /// Example: symbol_short!("TW_FOLL") for Twitter followers.
    pub metric_id: Symbol,
    /// Numeric threshold. For 50 k followers: 50_000.
    pub threshold: i128,
    /// How the live value is compared to the threshold.
    pub operator: ComparisonOperator,
}

/// Append-only verification record written each time the oracle is queried
/// and the KPI is confirmed. Front-ends / indexers can read this.
#[contracttype]
#[derive(Clone, Debug)]
pub struct KpiVerificationRecord {
    pub vault_id: u64,
    pub observed_value: i128,
    pub threshold: i128,
    pub verified_at: u64,   // ledger timestamp
    pub verified_by: Address, // caller that triggered verification
}

// ── Storage helpers ───────────────────────────────────────────────────────

pub fn get_kpi_config(env: &Env, vault_id: u64) -> Option<KpiOracleConfig> {
    env.storage().instance().get(&kpi_config_key(vault_id))
}

pub fn set_kpi_config(env: &Env, vault_id: u64, config: &KpiOracleConfig) {
    env.storage().instance().set(&kpi_config_key(vault_id), config);
}

/// Read the idempotent KPI-met flag. Returns `false` if never written.
pub fn is_kpi_met(env: &Env, vault_id: u64) -> bool {
    env.storage()
        .instance()
        .get(&kpi_met_key(vault_id))
        .unwrap_or(false)
}

/// Write-once setter — once `true` it can NEVER be set back.
/// Any attempt to call this when already `true` is a no-op (idempotent).
/// Any attempt to pass `false` after `true` panics — this is the core
/// invariant of the KPI engine.
pub fn set_kpi_met(env: &Env, vault_id: u64, value: bool) {
    let already_met = is_kpi_met(env, vault_id);

    if already_met && !value {
        panic!("KPI already verified: flag is write-once and cannot be unset");
    }

    // If already true and caller passes true again, it is a no-op.
    if already_met {
        return;
    }

    env.storage()
        .instance()
        .set(&kpi_met_key(vault_id), &value);
}

pub fn append_kpi_log(env: &Env, vault_id: u64, record: KpiVerificationRecord) {
    let key = kpi_log_key(vault_id);
    let mut log: Vec<KpiVerificationRecord> = env
        .storage()
        .instance()
        .get(&key)
        .unwrap_or(Vec::new(env));
    log.push_back(record);
    env.storage().instance().set(&key, &log);
}

pub fn get_kpi_log(env: &Env, vault_id: u64) -> Vec<KpiVerificationRecord> {
    env.storage()
        .instance()
        .get(&kpi_log_key(vault_id))
        .unwrap_or(Vec::new(env))
}

// ── Oracle query ──────────────────────────────────────────────────────────

/// Calls the configured oracle contract and returns the live metric value.
/// Uses Soroban's `invoke_contract` — same pattern as existing oracle.rs stubs.
/// Replace the placeholder with the real cross-contract call when your oracle
/// adapter contract is deployed.
fn query_oracle_value(env: &Env, config: &KpiOracleConfig) -> i128 {
    // Real call (uncomment when oracle adapter is ready):
    //
    // env.invoke_contract::<i128>(
    //     &config.oracle_contract,
    //     &Symbol::new(env, "query_kpi"),
    //     (config.metric_id.clone(),).into_val(env),
    // )
    //
    // Stub: returns 0 so the contract compiles and tests can mock via
    // `mock_all_auths` + a test oracle contract.
    let _ = &config.oracle_contract; // suppress unused warning
    let _ = &config.metric_id;
    0
}

fn compare(current: i128, threshold: i128, op: &ComparisonOperator) -> bool {
    match op {
        ComparisonOperator::GreaterThan          => current >  threshold,
        ComparisonOperator::GreaterThanOrEqual   => current >= threshold,
        ComparisonOperator::LessThan             => current <  threshold,
        ComparisonOperator::LessThanOrEqual      => current <= threshold,
        ComparisonOperator::Equal                => current == threshold,
    }
}

// ── Public verification entry-point ──────────────────────────────────────

/// Called by `kpi_vesting.rs` (and exposed as a public contract fn).
///
/// Flow:
///   1. If flag already `true`  → return `true` immediately (idempotent fast-path).
///   2. Query the oracle for the live metric value.
///   3. Evaluate the threshold comparison.
///   4. If condition met → flip flag (write-once), append log, emit event, return `true`.
///   5. If not met        → return `false` without touching the flag.
///
/// Panics if no KPI config has been set for this vault.
pub fn verify_kpi(env: &Env, vault_id: u64, caller: &Address) -> bool {
    // Fast-path: already verified, nothing to do.
    if is_kpi_met(env, vault_id) {
        return true;
    }

    let config = get_kpi_config(env, vault_id)
        .expect("KPI config not set for this vault");

    let live_value = query_oracle_value(env, &config);
    let condition_met = compare(live_value, config.threshold, &config.operator);

    if condition_met {
        // Write-once — cannot be undone after this point.
        set_kpi_met(env, vault_id, true);

        // Append immutable verification record.
        append_kpi_log(
            env,
            vault_id,
            KpiVerificationRecord {
                vault_id,
                observed_value: live_value,
                threshold: config.threshold,
                verified_at: env.ledger().timestamp(),
                verified_by: caller.clone(),
            },
        );

        // Emit event for indexers / front-end alerts.
        env.events().publish(
            (symbol_short!("KpiMet"), vault_id),
            (live_value, config.threshold, env.ledger().timestamp()),
        );
    }

    condition_met
}
