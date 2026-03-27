#![cfg(test)]

use soroban_sdk::{
    symbol_short, testutils::Address as _, Address, Env, Symbol,
};
use crate::kpi_engine::{
    attach_kpi_gate_internal, is_kpi_met, set_kpi_met, get_kpi_config,
    get_kpi_log, KpiOracleConfig,
};
use crate::kpi_vesting::{
    attach_kpi_gate, require_kpi_gate_passed, kpi_status, kpi_threshold,
};
use crate::oracle::ComparisonOperator;

// ── helpers ───────────────────────────────────────────────────────────────

fn make_env() -> Env {
    Env::default()
}

fn dummy_oracle(env: &Env) -> Address {
    Address::generate(env)
}

// ── idempotency tests ─────────────────────────────────────────────────────

#[test]
fn test_kpi_flag_starts_false() {
    let env = make_env();
    assert!(!is_kpi_met(&env, 1));
}

#[test]
fn test_kpi_flag_write_once_true() {
    let env = make_env();
    set_kpi_met(&env, 1, true);
    assert!(is_kpi_met(&env, 1));
}

#[test]
fn test_kpi_flag_idempotent_set_true_twice() {
    let env = make_env();
    // Setting true twice must not panic — it is a no-op on the second call.
    set_kpi_met(&env, 1, true);
    set_kpi_met(&env, 1, true); // no-op, must not panic
    assert!(is_kpi_met(&env, 1));
}

#[test]
#[should_panic(expected = "KPI already verified: flag is write-once and cannot be unset")]
fn test_kpi_flag_cannot_be_unset() {
    let env = make_env();
    set_kpi_met(&env, 1, true);
    set_kpi_met(&env, 1, false); // must panic
}

// ── config tests ──────────────────────────────────────────────────────────

#[test]
fn test_attach_kpi_gate_stores_config() {
    let env = make_env();
    let oracle = dummy_oracle(&env);

    attach_kpi_gate(
        &env,
        42,
        oracle.clone(),
        symbol_short!("TW_FOLL"),
        50_000,
        ComparisonOperator::GreaterThanOrEqual,
    );

    let cfg = get_kpi_config(&env, 42).expect("config should exist");
    assert_eq!(cfg.threshold, 50_000);
    assert_eq!(cfg.oracle_contract, oracle);
}

#[test]
#[should_panic(expected = "KPI threshold must be positive")]
fn test_attach_kpi_gate_rejects_zero_threshold() {
    let env = make_env();
    let oracle = dummy_oracle(&env);

    attach_kpi_gate(
        &env,
        1,
        oracle,
        symbol_short!("TW_FOLL"),
        0, // invalid
        ComparisonOperator::GreaterThanOrEqual,
    );
}

#[test]
#[should_panic(expected = "Cannot reconfigure a KPI gate that has already been verified")]
fn test_cannot_overwrite_verified_gate() {
    let env = make_env();
    let oracle = dummy_oracle(&env);

    attach_kpi_gate(
        &env,
        1,
        oracle.clone(),
        symbol_short!("TW_FOLL"),
        50_000,
        ComparisonOperator::GreaterThanOrEqual,
    );

    // Simulate KPI already met
    set_kpi_met(&env, 1, true);

    // Now trying to reconfigure must panic
    attach_kpi_gate(
        &env,
        1,
        oracle,
        symbol_short!("TW_FOLL"),
        99_000,
        ComparisonOperator::GreaterThanOrEqual,
    );
}

// ── gate enforcement tests ────────────────────────────────────────────────

#[test]
fn test_require_kpi_gate_passes_when_no_config() {
    let env = make_env();
    // No config set — gate is opt-in, must pass silently.
    require_kpi_gate_passed(&env, 99);
}

#[test]
#[should_panic(expected = "KPI gate not yet verified")]
fn test_require_kpi_gate_blocks_when_not_met() {
    let env = make_env();
    let oracle = dummy_oracle(&env);

    attach_kpi_gate(
        &env,
        5,
        oracle,
        symbol_short!("TW_FOLL"),
        50_000,
        ComparisonOperator::GreaterThanOrEqual,
    );

    // KPI not yet met — claim must be blocked.
    require_kpi_gate_passed(&env, 5);
}

#[test]
fn test_require_kpi_gate_passes_when_met() {
    let env = make_env();
    let oracle = dummy_oracle(&env);

    attach_kpi_gate(
        &env,
        5,
        oracle,
        symbol_short!("TW_FOLL"),
        50_000,
        ComparisonOperator::GreaterThanOrEqual,
    );

    set_kpi_met(&env, 5, true);

    // Should not panic now.
    require_kpi_gate_passed(&env, 5);
}

// ── threshold / status helpers ────────────────────────────────────────────

#[test]
fn test_kpi_threshold_returns_zero_when_no_config() {
    let env = make_env();
    assert_eq!(kpi_threshold(&env, 7), 0);
}

#[test]
fn test_kpi_threshold_returns_configured_value() {
    let env = make_env();
    let oracle = dummy_oracle(&env);

    attach_kpi_gate(
        &env,
        7,
        oracle,
        symbol_short!("TW_FOLL"),
        50_000,
        ComparisonOperator::GreaterThanOrEqual,
    );

    assert_eq!(kpi_threshold(&env, 7), 50_000);
}

#[test]
fn test_kpi_status_false_before_verification() {
    let env = make_env();
    assert!(!kpi_status(&env, 3));
}

#[test]
fn test_kpi_status_true_after_verification() {
    let env = make_env();
    set_kpi_met(&env, 3, true);
    assert!(kpi_status(&env, 3));
}

// ── log tests ─────────────────────────────────────────────────────────────

#[test]
fn test_verification_log_is_empty_before_verify() {
    let env = make_env();
    let log = get_kpi_log(&env, 10);
    assert_eq!(log.len(), 0);
}

#[test]
fn test_verification_log_appended_on_verify() {
    let env = make_env();
    let oracle = dummy_oracle(&env);
    let caller = Address::generate(&env);

    attach_kpi_gate(
        &env,
        10,
        oracle,
        symbol_short!("TW_FOLL"),
        50_000,
        ComparisonOperator::GreaterThanOrEqual,
    );

    // Manually flip flag and append log as verify_kpi would when oracle returns >= 50k.
    set_kpi_met(&env, 10, true);
    crate::kpi_engine::append_kpi_log(
        &env,
        10,
        crate::kpi_engine::KpiVerificationRecord {
            vault_id: 10,
            observed_value: 52_000,
            threshold: 50_000,
            verified_at: env.ledger().timestamp(),
            verified_by: caller,
        },
    );

    let log = get_kpi_log(&env, 10);
    assert_eq!(log.len(), 1);
    assert_eq!(log.get(0).unwrap().observed_value, 52_000);
}
