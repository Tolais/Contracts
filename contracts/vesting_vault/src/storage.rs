use soroban_sdk::{Env, Vec, Address, Map};
use crate::types::{ClaimEvent, AuthorizedPayoutAddress, AddressWhitelistRequest, Nullifier, Commitment, PathPaymentConfig, PathPaymentClaimEvent};

pub const CLAIM_HISTORY: &str = "CLAIM_HISTORY";
pub const AUTHORIZED_PAYOUT_ADDRESS: &str = "AUTHORIZED_PAYOUT_ADDRESS";
pub const PENDING_ADDRESS_REQUEST: &str = "PENDING_ADDRESS_REQUEST";

// Emergency pause storage keys
pub const AUDITORS: &str = "AUDITORS";
pub const AUDITOR_PAUSE_REQUESTS: &str = "AUDITOR_PAUSE_REQUESTS";
pub const EMERGENCY_PAUSE: &str = "EMERGENCY_PAUSE";

// Cross-project reputation storage keys
pub const REPUTATION_BRIDGE_CONTRACT: &str = "REPUTATION_BRIDGE_CONTRACT";
pub const REPUTATION_BONUS_APPLIED: &str = "REPUTATION_BONUS_APPLIED";

// Milestone vesting storage keys
pub const MILESTONE_CONFIGS: &str = "MILESTONE_CONFIGS";
pub const MILESTONE_STATUS: &str = "MILESTONE_STATUS";

// Zero-Knowledge Privacy Claims storage keys
pub const NULLIFIER_MAP: &str = "NULLIFIER_MAP";
pub const COMMITMENT_STORAGE: &str = "COMMITMENT_STORAGE";
pub const PRIVACY_CLAIM_HISTORY: &str = "PRIVACY_CLAIM_HISTORY";
pub const MERKLE_ROOTS: &str = "MERKLE_ROOTS";

// Stellar Horizon Path Payment Claim storage keys
pub const PATH_PAYMENT_CONFIG: &str = "PATH_PAYMENT_CONFIG";
pub const PATH_PAYMENT_CLAIM_HISTORY: &str = "PATH_PAYMENT_CLAIM_HISTORY";

// 48 hours in seconds
const TIMELOCK_DURATION: u64 = 172_800;

// 7 days in seconds for emergency pause
const EMERGENCY_PAUSE_DURATION: u64 = 604_800;

pub fn get_claim_history(e: &Env) -> Vec<ClaimEvent> {
    e.storage()
        .instance()
        .get(&CLAIM_HISTORY)
        .unwrap_or(Vec::new(e))
}

pub fn set_claim_history(e: &Env, history: &Vec<ClaimEvent>) {
    e.storage().instance().set(&CLAIM_HISTORY, history);
}

pub fn get_authorized_payout_address(e: &Env, beneficiary: &Address) -> Option<AuthorizedPayoutAddress> {
    e.storage()
        .instance()
        .get(&(AUTHORIZED_PAYOUT_ADDRESS, beneficiary))
}

pub fn set_authorized_payout_address(e: &Env, beneficiary: &Address, auth_address: &AuthorizedPayoutAddress) {
    e.storage().instance().set(&(AUTHORIZED_PAYOUT_ADDRESS, beneficiary), auth_address);
}

pub fn get_pending_address_request(e: &Env, beneficiary: &Address) -> Option<AddressWhitelistRequest> {
    e.storage()
        .instance()
        .get(& (PENDING_ADDRESS_REQUEST, beneficiary))
}

pub fn set_pending_address_request(e: &Env, beneficiary: &Address, request: &AddressWhitelistRequest) {
    e.storage().instance().set(&(PENDING_ADDRESS_REQUEST, beneficiary), request);
}

pub fn remove_pending_address_request(e: &Env, beneficiary: &Address) {
    e.storage().instance().remove(&(PENDING_ADDRESS_REQUEST, beneficiary));
}

pub fn get_timelock_duration() -> u64 {
    TIMELOCK_DURATION
}

// Emergency pause functions
pub fn get_auditors(e: &Env) -> Vec<Address> {
    e.storage()
        .instance()
        .get(&AUDITORS)
        .unwrap_or(Vec::new(e))
}

pub fn set_auditors(e: &Env, auditors: &Vec<Address>) {
    e.storage().instance().set(&AUDITORS, auditors);
}

pub fn get_auditor_pause_requests(e: &Env) -> Map<Address, crate::emergency::AuditorPauseRequest> {
    e.storage()
        .instance()
        .get(&AUDITOR_PAUSE_REQUESTS)
        .unwrap_or(Map::new(e))
}

pub fn set_auditor_pause_requests(e: &Env, requests: &Map<Address, crate::emergency::AuditorPauseRequest>) {
    e.storage().instance().set(&AUDITOR_PAUSE_REQUESTS, requests);
}

pub fn get_emergency_pause(e: &Env) -> Option<crate::emergency::EmergencyPause> {
    e.storage().instance().get(&EMERGENCY_PAUSE)
}

pub fn set_emergency_pause(e: &Env, pause: &crate::emergency::EmergencyPause) {
    e.storage().instance().set(&EMERGENCY_PAUSE, pause);
}

pub fn remove_emergency_pause(e: &Env) {
    e.storage().instance().remove(&EMERGENCY_PAUSE);
}

// Cross-project reputation functions
pub fn get_reputation_bridge_contract(e: &Env) -> Option<Address> {
    e.storage().instance().get(&REPUTATION_BRIDGE_CONTRACT)
}

pub fn set_reputation_bridge_contract(e: &Env, contract_address: &Address) {
    e.storage().instance().set(&REPUTATION_BRIDGE_CONTRACT, contract_address);
}

pub fn has_reputation_bonus_applied(e: &Env, beneficiary: &Address) -> bool {
    e.storage()
        .instance()
        .get(&(REPUTATION_BONUS_APPLIED, beneficiary))
        .unwrap_or(false)
}

pub fn set_reputation_bonus_applied(e: &Env, beneficiary: &Address) {
    e.storage().instance().set(&(REPUTATION_BONUS_APPLIED, beneficiary), &true);
}

// Milestone vesting functions
pub fn get_milestone_configs(e: &Env, vesting_id: u32) -> Option<Vec<u32>> {
    e.storage().instance().get(&(MILESTONE_CONFIGS, vesting_id))
}

pub fn set_milestone_configs(e: &Env, vesting_id: u32, milestones: &Vec<u32>) {
    e.storage().instance().set(&(MILESTONE_CONFIGS, vesting_id), milestones);
}

pub fn get_milestone_status(e: &Env, vesting_id: u32) -> Map<u32, bool> {
    e.storage()
        .instance()
        .get(&(MILESTONE_STATUS, vesting_id))
        .unwrap_or(Map::new(e))
}

pub fn set_milestone_status(e: &Env, vesting_id: u32, status: &Map<u32, bool>) {
    e.storage().instance().set(&(MILESTONE_STATUS, vesting_id), status);
}

pub fn get_emergency_pause_duration() -> u64 {
    EMERGENCY_PAUSE_DURATION
}

// Zero-Knowledge Privacy Claims functions

// Nullifier map functions - prevent double-spending in private claims
pub fn is_nullifier_used(e: &Env, nullifier: &Nullifier) -> bool {
    e.storage()
        .instance()
        .get(&(NULLIFIER_MAP, nullifier))
        .unwrap_or(false)
}

pub fn set_nullifier_used(e: &Env, nullifier: &Nullifier) {
    e.storage().instance().set(&(NULLIFIER_MAP, nullifier), &true);
}

// Commitment storage functions
pub fn get_commitment(e: &Env, commitment_hash: &[u8; 32]) -> Option<Commitment> {
    e.storage().instance().get(&(COMMITMENT_STORAGE, commitment_hash))
}

pub fn set_commitment(e: &Env, commitment_hash: &[u8; 32], commitment: &Commitment) {
    e.storage().instance().set(&(COMMITMENT_STORAGE, commitment_hash), commitment);
}

pub fn mark_commitment_used(e: &Env, commitment_hash: &[u8; 32]) {
    if let Some(mut commitment) = get_commitment(e, commitment_hash) {
        commitment.is_used = true;
        set_commitment(e, commitment_hash, &commitment);
    }
}

// Privacy claim history functions
pub fn get_privacy_claim_history(e: &Env) -> Vec<crate::types::PrivacyClaimEvent> {
    e.storage()
        .instance()
        .get(&PRIVACY_CLAIM_HISTORY)
        .unwrap_or(Vec::new(e))
}

pub fn add_privacy_claim_event(e: &Env, event: &crate::types::PrivacyClaimEvent) {
    let mut history = get_privacy_claim_history(e);
    history.push_back(event.clone());
    e.storage().instance().set(&PRIVACY_CLAIM_HISTORY, &history);
}

// Merkle root management for ZK proofs
pub fn add_merkle_root(e: &Env, merkle_root: &[u8; 32]) {
    let mut roots = get_merkle_roots(e);
    roots.push_back(*merkle_root);
    e.storage().instance().set(&MERKLE_ROOTS, &roots);
}

pub fn get_merkle_roots(e: &Env) -> Vec<[u8; 32]> {
    e.storage()
        .instance()
        .get(&MERKLE_ROOTS)
        .unwrap_or(Vec::new(e))
}

pub fn is_valid_merkle_root(e: &Env, merkle_root: &[u8; 32]) -> bool {
    let roots = get_merkle_roots(e);
    roots.contains(merkle_root)
}

// Stellar Horizon Path Payment Claim storage functions
pub fn get_path_payment_config(e: &Env) -> Option<PathPaymentConfig> {
    e.storage().instance().get(&PATH_PAYMENT_CONFIG)
}

pub fn set_path_payment_config(e: &Env, config: &PathPaymentConfig) {
    e.storage().instance().set(&PATH_PAYMENT_CONFIG, config);
}

pub fn get_path_payment_claim_history(e: &Env) -> Vec<PathPaymentClaimEvent> {
    e.storage()
        .instance()
        .get(&PATH_PAYMENT_CLAIM_HISTORY)
        .unwrap_or(Vec::new(e))
}

pub fn add_path_payment_claim_event(e: &Env, event: &PathPaymentClaimEvent) {
    let mut history = get_path_payment_claim_history(e);
    history.push_back(event.clone());
    e.storage().instance().set(&PATH_PAYMENT_CLAIM_HISTORY, &history);
}