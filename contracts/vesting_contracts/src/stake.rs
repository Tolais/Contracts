/// Auto-stake module for the Vesting Vault.
///
/// Tokens never leave the vault. Staking registers the vault's locked balance
/// as an active stake on a whitelisted external staking contract via a
/// synchronous cross-contract call. Yield accrues on the staking contract and
/// is pulled back to the beneficiary via `claim_yield`. Revocation always
/// unstakes before returning tokens to the treasury.
use soroban_sdk::{contracttype, contractevent, Address, Env, Symbol, Vec};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Tracks whether a vault's tokens are currently registered as a stake.
/// Soroban contracttype enums only support tuple or unit variants (no named fields).
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum StakeState {
    /// Tokens are not staked.
    Unstaked,
    /// Tokens are registered as a stake: (since_timestamp, staking_contract).
    Staked(u64, Address),
}

/// Per-vault staking metadata stored alongside the vault.
#[contracttype]
#[derive(Clone)]
pub struct VaultStakeInfo {
    /// How many tokens are currently registered as staked.
    pub tokens_staked: i128,
    /// Current stake state.
    pub stake_state: StakeState,
    /// Yield accumulated and not yet claimed (informational; authoritative value lives on staking contract).
    pub accumulated_yield: i128,
}

/// View type returned by `get_stake_status`.
#[contracttype]
#[derive(Clone)]
pub struct StakeStatusView {
    pub vault_id: u64,
    pub stake_state: StakeState,
    pub tokens_staked: i128,
    pub accumulated_yield: i128,
}

/// Dedicated error type for all staking-related failures.
///
/// Used as panic message strings so callers and tests can match on them.
/// In a future upgrade these can be surfaced as a proper `contracterror` enum.
#[contracttype]
#[derive(Clone)]
pub enum StakeError {
    /// Vault is already registered as a stake.
    AlreadyStaked,
    /// Vault is not currently staked.
    NotStaked,
    /// Vault has zero locked balance — nothing to stake.
    InsufficientBalance,
    /// The supplied staking contract address is not on the whitelist.
    UnauthorizedStakingContract,
    /// The vault has been revoked; yield can no longer be claimed.
    BeneficiaryRevoked,
    /// The cross-contract call to the staking contract failed.
    CrossContractCallFailed,
    /// Unstaking before revocation could not be completed.
    UnstakeBeforeRevocationFailed,
    /// The yield claim call to the staking contract failed.
    YieldClaimFailed,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// DataKey variant for per-vault stake info.
#[contracttype]
pub enum StakeDataKey {
    VaultStakeInfo(u64),
    ApprovedStakingContracts,
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

pub fn get_stake_info(env: &Env, vault_id: u64) -> VaultStakeInfo {
    env.storage()
        .instance()
        .get(&StakeDataKey::VaultStakeInfo(vault_id))
        .unwrap_or(VaultStakeInfo {
            tokens_staked: 0,
            stake_state: StakeState::Unstaked,
            accumulated_yield: 0,
        })
}

pub fn set_stake_info(env: &Env, vault_id: u64, info: &VaultStakeInfo) {
    env.storage()
        .instance()
        .set(&StakeDataKey::VaultStakeInfo(vault_id), info);
}

/// Returns the list of whitelisted staking contract addresses.
pub fn get_approved_staking_contracts(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&StakeDataKey::ApprovedStakingContracts)
        .unwrap_or(Vec::new(env))
}

/// Adds `contract` to the staking contract whitelist.
pub fn add_approved_staking_contract(env: &Env, contract: Address) {
    let mut list = get_approved_staking_contracts(env);
    if !list.contains(&contract) {
        list.push_back(contract);
        env.storage()
            .instance()
            .set(&StakeDataKey::ApprovedStakingContracts, &list);
    }
}

/// Removes `contract` from the staking contract whitelist.
pub fn remove_approved_staking_contract(env: &Env, contract: Address) {
    let list = get_approved_staking_contracts(env);
    let mut updated = Vec::new(env);
    for c in list.iter() {
        if c != contract {
            updated.push_back(c);
        }
    }
    env.storage()
        .instance()
        .set(&StakeDataKey::ApprovedStakingContracts, &updated);
}

/// Returns `true` if `contract` is on the whitelist.
pub fn is_approved_staking_contract(env: &Env, contract: &Address) -> bool {
    get_approved_staking_contracts(env).contains(contract)
}

// ---------------------------------------------------------------------------
// Staking contract cross-contract call helpers
//
// Soroban cross-contract calls use `soroban_sdk::invoke_contract` with a
// symbol for the function name and a Vec<Val> for arguments. We wrap each
// call in a typed helper so callers don't deal with raw Val encoding.
// ---------------------------------------------------------------------------

use soroban_sdk::{IntoVal, Val};

/// Call `stake_tokens(beneficiary, vault_id, amount)` on the staking contract.
pub fn call_stake_tokens(
    env: &Env,
    staking_contract: &Address,
    beneficiary: &Address,
    vault_id: u64,
    amount: i128,
) {
    let args: soroban_sdk::Vec<Val> = soroban_sdk::vec![
        env,
        beneficiary.into_val(env),
        vault_id.into_val(env),
        amount.into_val(env),
    ];
    env.invoke_contract::<()>(staking_contract, &Symbol::new(env, "stake_tokens"), args);
}

/// Call `unstake_tokens(beneficiary, vault_id)` on the staking contract.
pub fn call_unstake_tokens(
    env: &Env,
    staking_contract: &Address,
    beneficiary: &Address,
    vault_id: u64,
) {
    let args: soroban_sdk::Vec<Val> = soroban_sdk::vec![
        env,
        beneficiary.into_val(env),
        vault_id.into_val(env),
    ];
    env.invoke_contract::<()>(staking_contract, &Symbol::new(env, "unstake_tokens"), args);
}

/// Call `claim_yield_for(beneficiary, vault_id)` on the staking contract and
/// return the yield amount.
pub fn call_claim_yield_for(
    env: &Env,
    staking_contract: &Address,
    beneficiary: &Address,
    vault_id: u64,
) -> i128 {
    let args: soroban_sdk::Vec<Val> = soroban_sdk::vec![
        env,
        beneficiary.into_val(env),
        vault_id.into_val(env),
    ];
    env.invoke_contract::<i128>(staking_contract, &Symbol::new(env, "claim_yield_for"), args)
}

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

pub fn emit_staked(env: &Env, vault_id: u64, beneficiary: &Address, amount: i128, staking_contract: &Address) {
    env.events().publish((Symbol::new(env, "auto_staked"), vault_id), (beneficiary.clone(), amount, staking_contract.clone()));
}

pub fn emit_unstaked(env: &Env, vault_id: u64, beneficiary: &Address, amount: i128) {
    env.events().publish((Symbol::new(env, "auto_unstaked"), vault_id), (beneficiary.clone(), amount));
}

pub fn emit_yield_claimed(env: &Env, vault_id: u64, beneficiary: &Address, amount: i128) {
    env.events().publish((Symbol::new(env, "yield_claimed"), vault_id), (beneficiary.clone(), amount));
}

pub fn emit_revocation_unstaked(env: &Env, vault_id: u64, beneficiary: &Address) {
    env.events().publish((Symbol::new(env, "revoc_unstaked"), vault_id), beneficiary.clone());
}

// Typed contract events for staking operations. Using `#[contractevent]`
// ensures the event payload implements the runtime conversion traits and
// removes the deprecated untyped `env.events().publish((Symbol,..), ..)` calls.
#[contractevent]
pub struct AutoStakedEvent {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
    pub staking_contract: Address,
}

#[contractevent]
pub struct AutoUnstakedEvent {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
}

#[contractevent]
pub struct YieldClaimedEvent {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub amount: i128,
}

#[contractevent]
pub struct RevocationUnstakedEvent {
    pub vault_id: u64,
    pub beneficiary: Address,
}
