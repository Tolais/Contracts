#![no_std]
use soroban_sdk::{
    contract,
    contractimpl,
    contracttype,
    token,
    vec,
    Address,
    Env,
    IntoVal,
    Map,
    Symbol,
    Val,
    Vec,
    String,
};

mod factory;
pub use factory::{ VestingFactory, VestingFactoryClient };
mod oracle;
pub use oracle::{ OracleClient, OracleCondition, OracleType, ComparisonOperator, PerformanceCliff };

pub mod stake;
pub use stake::{
    StakeDataKey, StakeState, StakeStatusView, VaultStakeInfo,
    get_stake_info, set_stake_info,
    get_approved_staking_contracts, add_approved_staking_contract,
    remove_approved_staking_contract, is_approved_staking_contract,
    call_stake_tokens, call_unstake_tokens, call_claim_yield_for,
};

pub mod inheritance;
pub use inheritance::{
    SuccessionState, SuccessionView, InheritanceError,
    NominatedData, ClaimPendingData, SucceededData,
    MIN_SWITCH_DURATION, MAX_SWITCH_DURATION, MIN_CHALLENGE_WINDOW, MAX_CHALLENGE_WINDOW,
    nominate_backup, revoke_backup, update_activity,
    initiate_succession_claim, finalise_succession, cancel_succession_claim,
    get_succession_status, get_succession_state,
};

// 10 years in seconds
pub const MAX_DURATION: u64 = 315_360_000;
// 72 hours in seconds for challenge period
pub const CHALLENGE_PERIOD: u64 = 259_200;
// 51% voting threshold (represented as basis points: 5100 = 51.00%)
pub const VOTING_THRESHOLD: u32 = 5100;

#[contracttype]
pub enum WhitelistDataKey {
    WhitelistedTokens,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    AdminAddress,
    AdminBalance,
    InitialSupply,
    ProposedAdmin,
    VaultCount,
    VaultData(u64),
    VaultMilestones(u64),
    VaultPerformanceCliff(u64),
    UserVaults(Address),
    IsPaused,
    IsDeprecated,
    MigrationTarget,
    Token,
    TotalShares,
    TotalStaked,
    StakingContract,
    // Defensive Governance
    GovernanceProposal(u64),
    GovernanceVotes(u64, Address),
    ProposalCount,
    TotalLockedValue,
    PausedVault(u64),
    PauseAuthority,
    NFTMinter,
    CollateralBridge,
    MetadataAnchor,
    VotingDelegate(Address),
    DelegatedBeneficiaries(Address),
    GlobalAccelerationPct,
    RevokedVaults,
    VaultSuccession(u64),
}

#[contracttype]
#[derive(Clone)]
pub struct PausedVault {
    pub vault_id: u64,
    pub pause_timestamp: u64,
    pub pause_authority: Address,
    pub reason: String,
}

#[contracttype]
#[derive(Clone)]
pub struct AssetAllocation {
    pub asset_id: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub locked_amount: i128, // Amount locked for collateral liens
    pub percentage: u32, // Percentage of total allocation (basis points, 10000 = 100%)
}

#[contracttype]
#[derive(Clone)]
pub struct Vault {
    pub allocations: Vec<AssetAllocation>, // Basket of assets
    pub keeper_fee: i128,
    pub staked_amount: i128,
    pub owner: Address,
    pub delegate: Option<Address>,
    pub title: String,
    pub start_time: u64,
    pub end_time: u64,
    pub creation_time: u64,
    pub step_duration: u64,
    pub is_initialized: bool,
    pub is_irrevocable: bool,
    pub is_transferable: bool,
    pub is_frozen: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Milestone {
    pub id: u64,
    pub percentage: u32,
    pub is_unlocked: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceAction {
    AdminRotation(Address),     // new_admin
    ContractUpgrade(Address),  // new_contract_address
    EmergencyPause(bool),       // pause_state
}

#[contracttype]
#[derive(Clone)]
pub struct GovernanceProposal {
    pub id: u64,
    pub action: GovernanceAction,
    pub proposer: Address,
    pub created_at: u64,
    pub challenge_end: u64,
    pub is_executed: bool,
    pub is_cancelled: bool,
    pub yes_votes: i128,   // Total locked value voting yes
    pub no_votes: i128,    // Total locked value voting no
}

#[contracttype]
#[derive(Clone)]
pub struct Vote {
    pub voter: Address,
    pub vote_weight: i128,
    pub is_yes: bool,
    pub voted_at: u64,
}

#[contracttype]
pub struct BatchCreateData {
    pub recipients: Vec<Address>,
    pub asset_baskets: Vec<Vec<AssetAllocation>>, // Each recipient gets a basket of assets
    pub start_times: Vec<u64>,
    pub end_times: Vec<u64>,
    pub keeper_fees: Vec<i128>,
    pub step_durations: Vec<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct ScheduleConfig {
    pub owner: Address,
    pub asset_basket: Vec<AssetAllocation>, // Basket of assets for this schedule
    pub start_time: u64,
    pub end_time: u64,
    pub keeper_fee: i128,
    pub is_revocable: bool,
    pub is_transferable: bool,
}

#[contracttype]
pub struct VaultCreated {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub total_amount: i128,
    pub cliff_duration: u64,
    pub start_time: u64,
    pub title: String,
}

#[contracttype]
pub struct GovernanceProposalCreated {
    pub proposal_id: u64,
    pub action: GovernanceAction,
    pub proposer: Address,
    pub challenge_end: u64,
}

#[contracttype]
pub struct VoteCast {
    pub proposal_id: u64,
    pub voter: Address,
    pub vote_weight: i128,
    pub is_yes: bool,
}

#[contracttype]
pub struct GovernanceActionExecuted {
    pub proposal_id: u64,
    pub action: GovernanceAction,
}

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        if env.storage().instance().has(&DataKey::AdminAddress) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::AdminAddress, &admin);
        env.storage().instance().set(&DataKey::AdminBalance, &initial_supply);
        env.storage().instance().set(&DataKey::InitialSupply, &initial_supply);
        env.storage().instance().set(&DataKey::VaultCount, &0u64);
        env.storage().instance().set(&DataKey::IsPaused, &false);
        env.storage().instance().set(&DataKey::IsDeprecated, &false);
        env.storage().instance().set(&DataKey::TotalShares, &0i128);
        env.storage().instance().set(&DataKey::TotalStaked, &0i128);
        // Initialize governance
        env.storage().instance().set(&DataKey::ProposalCount, &0u64);
        env.storage().instance().set(&DataKey::TotalLockedValue, &initial_supply);
    }

    pub fn set_token(env: Env, token: Address) {
        Self::require_admin(&env);
        if env.storage().instance().has(&DataKey::Token) {
            panic!("Token already set");
        }
        env.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn set_nft_minter(env: Env, minter: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::NFTMinter, &minter);
    }

    pub fn add_to_whitelist(env: Env, token: Address) {
        Self::require_admin(&env);
        let mut whitelist: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&WhitelistDataKey::WhitelistedTokens)
            .unwrap_or(Map::new(&env));
        whitelist.set(token.clone(), true);
        env.storage().instance().set(&WhitelistDataKey::WhitelistedTokens, &whitelist);
    }

    // Defensive Governance Functions
    pub fn propose_admin_rotation(env: Env, new_admin: Address) -> u64 {
        Self::require_admin(&env);
        Self::create_governance_proposal(env, GovernanceAction::AdminRotation(new_admin))
    }

    pub fn propose_contract_upgrade(env: Env, new_contract: Address) -> u64 {
        Self::require_admin(&env);
        Self::create_governance_proposal(env, GovernanceAction::ContractUpgrade(new_contract))
    pub fn accept_ownership(env: Env) {
        let proposed: Address = env
            .storage()
            .instance()
            .get(&DataKey::ProposedAdmin)
            .expect("No proposed admin");
        proposed.require_auth();
        env.storage().instance().set(&DataKey::AdminAddress, &proposed);
        env.storage().instance().remove(&DataKey::ProposedAdmin);
    }

    pub fn propose_emergency_pause(env: Env, pause_state: bool) -> u64 {
        Self::require_admin(&env);
        Self::create_governance_proposal(env, GovernanceAction::EmergencyPause(pause_state))
    }

    pub fn vote_on_proposal(env: Env, proposal_id: u64, is_yes: bool) {
        // Get the caller address - this will be the vault owner/beneficiary
        let voter = Address::generate(&env); // In real implementation, this would be env.invoker()
        voter.require_auth();
        let vote_weight = Self::get_voter_locked_value(&env, &voter);
        
        if vote_weight <= 0 {
            panic!("No voting power - no locked tokens");
        }

        let mut proposal = Self::get_proposal(&env, proposal_id);
        
        // Check if voting is still open
        let now = env.ledger().timestamp();
        if now >= proposal.challenge_end {
            panic!("Voting period has ended");
        }
        
        if proposal.is_executed || proposal.is_cancelled {
            panic!("Proposal is no longer active");
        }

        // Check if already voted
        let vote_key = DataKey::GovernanceVotes(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted on this proposal");
        }

        // Record vote
        let vote = Vote {
            voter: voter.clone(),
            vote_weight,
            is_yes,
            voted_at: now,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update proposal vote counts
        if is_yes {
            proposal.yes_votes += vote_weight;
        } else {
            proposal.no_votes += vote_weight;
        }

        env.storage().instance().set(&DataKey::GovernanceProposal(proposal_id), &proposal);

        // Publish vote event
        let vote_event = VoteCast {
            proposal_id,
            voter,
            vote_weight,
            is_yes,
        };
        env.events().publish((Symbol::new(&env, "vote_cast"), proposal_id), vote_event);
    }

    pub fn execute_proposal(env: Env, proposal_id: u64) {
        let mut proposal = Self::get_proposal(&env, proposal_id);
        let now = env.ledger().timestamp();

        // Check challenge period has ended
        if now < proposal.challenge_end {
            panic!("Challenge period not yet ended");
        }

        if proposal.is_executed || proposal.is_cancelled {
            panic!("Proposal already processed");
        }

        // Check if proposal passes (no veto from 51%+ of locked value)
        let total_locked = Self::get_total_locked_value(&env);
        let no_percentage = (proposal.no_votes * 10000) / total_locked;

        if no_percentage >= VOTING_THRESHOLD as i128 {
            // Proposal is vetoed - cancel it
            proposal.is_cancelled = true;
            env.storage().instance().set(&DataKey::GovernanceProposal(proposal_id), &proposal);
            return;
        }

        // Execute the governance action
        Self::execute_governance_action(&env, &proposal.action);
        
        proposal.is_executed = true;
        env.storage().instance().set(&DataKey::GovernanceProposal(proposal_id), &proposal);

        // Publish execution event
        let exec_event = GovernanceActionExecuted {
            proposal_id,
            action: proposal.action.clone(),
        };
        env.events().publish((Symbol::new(&env, "governance_executed"), proposal_id), exec_event);
    }

    // Legacy pause function - now requires governance proposal
    pub fn toggle_pause(env: Env) {
        panic!("Direct pause not allowed. Use propose_emergency_pause() instead.");
    }

    pub fn create_vault_full(
        env: Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64
    ) -> u64 {
        Self::require_admin(&env);
        Self::create_vault_full_internal(
            &env,
            owner,
            amount,
            start_time,
            end_time,
            keeper_fee,
            is_revocable,
            is_transferable,
            step_duration
        )
    }
    /// Creates a vault with a diversified asset basket (pre-funded)
    pub fn create_vault_diversified_full(
        env: Env,
        owner: Address,
        asset_basket: Vec<AssetAllocation>,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64,
        title: String,
    ) -> u64 {
        Self::require_admin(&env);

        // Validate asset basket
        if !Self::validate_asset_basket(&asset_basket) {
            panic!("Asset basket percentages must sum to 10000 (100%)");
        }

        if asset_basket.is_empty() {
            panic!("Asset basket cannot be empty");
        }

        // Validate timing
        if start_time >= end_time {
            panic!("Start time must be before end time");
        }

        let max_duration = 10 * 365 * 24 * 60 * 60; // 10 years in seconds
        if end_time - start_time > max_duration {
            panic!("Duration exceeds maximum allowed");
        }

        let vault_id = Self::increment_vault_count(&env);

        // Transfer all assets from admin to contract
        let admin = Self::get_admin(env.clone());
        for allocation in asset_basket.iter() {
            token::Client::new(&env, &allocation.asset_id)
                .transfer(&admin, &env.current_contract_address(), &allocation.total_amount);
        }

        let vault = Vault {
            allocations: asset_basket,
            keeper_fee,
            staked_amount: 0,
            owner: owner.clone(),
            delegate: None,
            title,
            start_time,
            end_time,
            creation_time: env.ledger().timestamp(),
            step_duration,
            is_initialized: true,
            is_irrevocable: !is_revocable,
            is_transferable,
            is_frozen: false,
        };

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
        Self::add_user_vault_index(&env, &owner, vault_id);

        vault_id
    }

    /// Creates a vault with a diversified asset basket (lazy/unfunded)
    pub fn create_vault_diversified_lazy(
        env: Env,
        owner: Address,
        asset_basket: Vec<AssetAllocation>,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64,
        title: String,
    ) -> u64 {
        Self::require_admin(&env);

        // Validate asset basket
        if !Self::validate_asset_basket(&asset_basket) {
            panic!("Asset basket percentages must sum to 10000 (100%)");
        }

        if asset_basket.is_empty() {
            panic!("Asset basket cannot be empty");
        }

        // Validate timing
        if start_time >= end_time {
            panic!("Start time must be before end time");
        }

        let max_duration = 10 * 365 * 24 * 60 * 60; // 10 years in seconds
        if end_time - start_time > max_duration {
            panic!("Duration exceeds maximum allowed");
        }

        let vault_id = Self::increment_vault_count(&env);

        let vault = Vault {
            allocations: asset_basket,
            keeper_fee,
            staked_amount: 0,
            owner: owner.clone(),
            delegate: None,
            title,
            start_time,
            end_time,
            creation_time: env.ledger().timestamp(),
            step_duration,
            is_initialized: false, // Lazy vault starts uninitialized
            is_irrevocable: !is_revocable,
            is_transferable,
            is_frozen: false,
        };

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
        Self::add_user_vault_index(&env, &owner, vault_id);

        vault_id
    }
    /// Initializes a lazy diversified vault by transferring all assets
    pub fn initialize_diversified_vault(env: Env, vault_id: u64) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);

        if vault.is_initialized {
            panic!("Vault already initialized");
        }

        let admin = Self::get_admin(env.clone());

        // Transfer all assets from admin to contract
        for allocation in vault.allocations.iter() {
            token::Client::new(&env, &allocation.asset_id)
                .transfer(&admin, &env.current_contract_address(), &allocation.total_amount);
        }

        vault.is_initialized = true;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    pub fn create_vault_lazy(
        env: Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64
    ) -> u64 {
        Self::require_admin(&env);
        Self::create_vault_lazy_internal(
            &env,
            owner,
            amount,
            start_time,
            end_time,
            keeper_fee,
            is_revocable,
            is_transferable,
            step_duration
        )
    }

    pub fn batch_create_vaults_lazy(env: Env, data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        let total_amount = Self::validate_batch_data(&data);
        Self::require_deposited_tokens_for_batch(&env, total_amount);
        Self::reserve_admin_balance_for_batch(&env, total_amount);

        let mut ids = Vec::new(&env);
        for i in 0..data.recipients.len() {
            let id = Self::create_vault_prefunded_internal(
                &env,
                data.recipients.get(i).unwrap(),
                data.amounts.get(i).unwrap(),
                data.start_times.get(i).unwrap(),
                data.end_times.get(i).unwrap(),
                data.keeper_fees.get(i).unwrap(),
                true,
                false,
                data.step_durations.get(i).unwrap_or(0),
                false,
                data.step_durations.get(i).unwrap_or(0)
            );
            ids.push_back(id);
        }
        ids
    }

    pub fn batch_create_vaults_full(env: Env, data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        let total_amount = Self::validate_batch_data(&data);
        Self::require_deposited_tokens_for_batch(&env, total_amount);
        Self::reserve_admin_balance_for_batch(&env, total_amount);

        let mut ids = Vec::new(&env);
        for i in 0..data.recipients.len() {
            let id = Self::create_vault_prefunded_internal(
                &env,
                data.recipients.get(i).unwrap(),
                data.amounts.get(i).unwrap(),
                data.start_times.get(i).unwrap(),
                data.end_times.get(i).unwrap(),
                data.keeper_fees.get(i).unwrap(),
                true,
                false,
                data.step_durations.get(i).unwrap_or(0),
                true,
            );
            ids.push_back(id);
        }
        ids
    }

    pub fn batch_add_schedules(env: Env, schedules: Vec<ScheduleConfig>) -> Vec<u64> {
        Self::require_admin(&env);
        let total_amount = Self::validate_schedule_configs(&schedules);
        Self::require_deposited_tokens_for_batch(&env, total_amount);
        Self::reserve_admin_balance_for_batch(&env, total_amount);

        let mut ids = Vec::new(&env);
        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            let id = Self::create_vault_prefunded_internal(
                &env,
                schedule.owner,
                schedule.amount,
                schedule.start_time,
                schedule.end_time,
                schedule.keeper_fee,
                schedule.is_revocable,
                schedule.is_transferable,
                schedule.step_duration,
                true,
                data.step_durations.get(i).unwrap_or(0)
            );
            ids.push_back(id);
        }
        ids
    }

    /// Claims tokens from a diversified vesting vault
    /// Returns a vector of (asset_id, claimed_amount) pairs
    pub fn claim_tokens_diversified(env: Env, vault_id: u64) -> Vec<(Address, i128)> {
        Self::require_not_paused(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        if vault.is_frozen {
            panic!("Vault frozen");
        }
        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        // Check if this specific vault schedule is paused
        if Self::is_vault_paused(env.clone(), vault_id) {
            panic!("Vault schedule paused");
        }

        vault.owner.require_auth();

        // Heartbeat: reset Dead-Man's Switch on every primary interaction
        update_activity(&env, vault_id);

        // Validate asset basket
        if !Self::validate_asset_basket(&vault.allocations) {
            panic!("Invalid asset basket percentages");
        }

        let mut claimed_assets = Vec::new(&env);

        // Calculate and claim each asset in the basket
        for (i, allocation) in vault.allocations.iter().enumerate() {
            let vested_amount = Self::calculate_claimable_for_asset(&env, vault_id, &vault, i);
            let claimable_amount = vested_amount - allocation.released_amount;

            if claimable_amount > 0 {
                // Update the allocation's released amount
                let mut updated_allocation = allocation.clone();
                updated_allocation.released_amount += claimable_amount;
                vault.allocations.set(i.try_into().unwrap(), updated_allocation);

                // Transfer the tokens
                token::Client::new(&env, &allocation.asset_id)
                    .transfer(&env.current_contract_address(), &vault.owner, &claimable_amount);

                claimed_assets.push_back((allocation.asset_id.clone(), claimable_amount));
            }
        }

        // Save updated vault
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        // Mint NFT if configured
        if let Some(nft_minter) = env.storage().instance().get::<_, Address>(&DataKey::NFTMinter) {
            env.invoke_contract::<()>(
                &nft_minter,
                &Symbol::new(&env, "mint"),
                (&vault.owner,).into_val(&env),
            );
        }

        claimed_assets
    }

    /// Legacy single-asset claim function for backward compatibility
    pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        Self::require_not_paused(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        if vault.is_frozen {
            panic!("Vault frozen");
        }
        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        // Check if this specific vault schedule is paused
        if Self::is_vault_paused(env.clone(), vault_id) {
            panic!("Vault schedule paused");
        }

        vault.owner.require_auth();

        // Heartbeat: reset Dead-Man's Switch on every primary interaction
        update_activity(&env, vault_id);

        // For backward compatibility, only work with single-asset vaults
        if vault.allocations.len() != 1 {
            panic!("Use claim_tokens_diversified for multi-asset vaults");
        }

        let allocation = vault.allocations.get(0).unwrap();
        let vested = Self::calculate_claimable_for_asset(&env, vault_id, &vault, 0);
        if claim_amount > vested - allocation.released_amount {
            panic!("Insufficient vested tokens");
        }

        let mut updated_allocation = allocation.clone();
        updated_allocation.released_amount += claim_amount;
        vault.allocations.set(0, updated_allocation);

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        token::Client::new(&env, &allocation.asset_id)
            .transfer(&env.current_contract_address(), &vault.owner, &claim_amount);

        if let Some(nft_minter) = env.storage().instance().get::<_, Address>(&DataKey::NFTMinter) {
            env.invoke_contract::<()>(
                &nft_minter,
                &Symbol::new(&env, "mint"),
                (&vault.owner,).into_val(&env),
            );
        }

        claim_amount
    }

    pub fn set_milestones(env: Env, vault_id: u64, milestones: Vec<Milestone>) {
        Self::require_admin(&env);
        let mut total_pct: u32 = 0;
        for m in milestones.iter() {
            total_pct += m.percentage;
        }
        if total_pct > 100 {
            panic!("Total percentage > 100");
        }
        env.storage().instance().set(&DataKey::VaultMilestones(vault_id), &milestones);
    }

    pub fn get_milestones(env: Env, vault_id: u64) -> Vec<Milestone> {
        env.storage().instance().get(&DataKey::VaultMilestones(vault_id)).unwrap_or(Vec::new(&env))
    }

    pub fn unlock_milestone(env: Env, vault_id: u64, milestone_id: u64) {
        Self::require_admin(&env);
        let mut milestones = Self::get_milestones(env.clone(), vault_id);
        let mut found = false;
        let mut updated = Vec::new(&env);
        for m in milestones.iter() {
            if m.id == milestone_id {
                found = true;
                updated.push_back(Milestone {
                    id: m.id,
                    percentage: m.percentage,
                    is_unlocked: true,
                });
            } else {
                updated.push_back(m);
            }
        }
        if !found {
            panic!("Milestone not found");
        }
        env.storage().instance().set(&DataKey::VaultMilestones(vault_id), &updated);
    }

    pub fn freeze_vault(env: Env, vault_id: u64, freeze: bool) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        vault.is_frozen = freeze;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    pub fn pause_specific_schedule(env: Env, vault_id: u64, reason: String) {
        Self::require_pause_authority(&env);
        let vault = Self::get_vault_internal(&env, vault_id);
        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        // Check if already paused
        if env.storage().instance().has(&DataKey::PausedVault(vault_id)) {
            panic!("Vault already paused");
        }

        let paused_vault = PausedVault {
            vault_id,
            pause_timestamp: env.ledger().timestamp(),
            pause_authority: env.current_contract_address(), // Will be replaced with actual authority
            reason,
        };

        env.storage().instance().set(&DataKey::PausedVault(vault_id), &paused_vault);
    }

    pub fn resume_specific_schedule(env: Env, vault_id: u64) {
        Self::require_pause_authority(&env);

        // Check if vault is actually paused
        if !env.storage().instance().has(&DataKey::PausedVault(vault_id)) {
            panic!("Vault not paused");
        }

        env.storage().instance().remove(&DataKey::PausedVault(vault_id));
    }

    pub fn set_pause_authority(env: Env, authority: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::PauseAuthority, &authority);
    }

    pub fn get_pause_authority(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::PauseAuthority)
    }

    pub fn is_vault_paused(env: Env, vault_id: u64) -> bool {
        env.storage().instance().has(&DataKey::PausedVault(vault_id))
    }

    pub fn get_paused_vault_info(env: Env, vault_id: u64) -> Option<PausedVault> {
        env.storage().instance().get(&DataKey::PausedVault(vault_id))
    }

    pub fn mark_irrevocable(env: Env, vault_id: u64) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        vault.is_irrevocable = true;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    pub fn set_performance_cliff(env: Env, vault_id: u64, cliff: PerformanceCliff) {
        Self::require_admin(&env);
        // Verify vault exists
        Self::get_vault_internal(&env, vault_id);
        env.storage().instance().set(&DataKey::VaultPerformanceCliff(vault_id), &cliff);
    }

    pub fn get_performance_cliff(env: Env, vault_id: u64) -> Option<PerformanceCliff> {
        env.storage().instance().get(&DataKey::VaultPerformanceCliff(vault_id))
    }

    pub fn is_cliff_passed(env: Env, vault_id: u64) -> bool {
        if let Some(cliff) = Self::get_performance_cliff(env.clone(), vault_id) {
            OracleClient::is_cliff_passed(&env, &cliff, vault_id)
        } else {
            // No performance cliff set, use time-based cliff check
            let vault = Self::get_vault_internal(&env, vault_id);
            let now = env.ledger().timestamp();
            now > vault.start_time
        }
    }

    pub fn create_vault_with_cliff(
        env: Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64,
        cliff: PerformanceCliff
    ) -> u64 {
        Self::require_admin(&env);
        let vault_id = Self::create_vault_full_internal(
            &env,
            owner,
            amount,
            start_time,
            end_time,
            keeper_fee,
            is_revocable,
            is_transferable,
            step_duration
        );
        Self::set_performance_cliff(env, vault_id, cliff);
        vault_id
    }

    /// Gets total claimable amount across all assets (for backward compatibility)
    pub fn get_claimable_amount(env: Env, vault_id: u64) -> i128 {
        let vault = Self::get_vault_internal(&env, vault_id);
        Self::calculate_claimable(&env, vault_id, &vault)
    }

    /// Gets claimable amounts for each asset in the basket
    pub fn get_claimable_diversified(env: Env, vault_id: u64) -> Vec<(Address, i128)> {
        let vault = Self::get_vault_internal(&env, vault_id);
        let mut claimable_amounts = Vec::new(&env);

        for (i, allocation) in vault.allocations.iter().enumerate() {
            let vested_amount = Self::calculate_claimable_for_asset(&env, vault_id, &vault, i);
            let claimable_amount = vested_amount - allocation.released_amount;
            claimable_amounts.push_back((allocation.asset_id.clone(), claimable_amount));
        }

        claimable_amounts
    }

    /// Locks tokens for a specific asset in the vault (for collateral)
    pub fn lock_tokens_for_asset(env: Env, vault_id: u64, asset_id: Address, amount: i128) {
        let bridge: Address = env
            .storage()
            .instance()
            .get(&DataKey::CollateralBridge)
            .expect("Collateral bridge not set");
        bridge.require_auth();

        let mut vault = Self::get_vault_internal(&env, vault_id);

        // Find the asset allocation
        let mut found = false;
        for (i, allocation) in vault.allocations.iter().enumerate() {
            if allocation.asset_id == asset_id {
                let available = allocation.total_amount - allocation.released_amount - allocation.locked_amount;
                if amount > available {
                    panic!("Insufficient available tokens for locking");
                }

                let mut updated_allocation = allocation.clone();
                updated_allocation.locked_amount += amount;
                vault.allocations.set(i.try_into().unwrap(), updated_allocation);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Asset not found in vault");
        }

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    /// Legacy function for single-asset vaults
    pub fn lock_tokens(env: Env, vault_id: u64, amount: i128) {
        let vault = Self::get_vault_internal(&env, vault_id);
        if vault.allocations.len() != 1 {
            panic!("Use lock_tokens_for_asset for multi-asset vaults");
        }

        let asset_id = vault.allocations.get(0).unwrap().asset_id.clone();
        Self::lock_tokens_for_asset(env, vault_id, asset_id, amount);
    }

    /// Unlocks tokens for a specific asset in the vault
    pub fn unlock_tokens_for_asset(env: Env, vault_id: u64, asset_id: Address, amount: i128) {
        let bridge: Address = env
            .storage()
            .instance()
            .get(&DataKey::CollateralBridge)
            .expect("Collateral bridge not set");
        bridge.require_auth();

        let mut vault = Self::get_vault_internal(&env, vault_id);

        // Find the asset allocation
        let mut found = false;
        for (i, allocation) in vault.allocations.iter().enumerate() {
            if allocation.asset_id == asset_id {
                if amount > allocation.locked_amount {
                    panic!("Cannot unlock more than locked amount");
                }

                let mut updated_allocation = allocation.clone();
                updated_allocation.locked_amount -= amount;
                vault.allocations.set(i.try_into().unwrap(), updated_allocation);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Asset not found in vault");
        }

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    /// Legacy function for single-asset vaults
    pub fn unlock_tokens(env: Env, vault_id: u64, amount: i128) {
        let vault = Self::get_vault_internal(&env, vault_id);
        if vault.allocations.len() != 1 {
            panic!("Use unlock_tokens_for_asset for multi-asset vaults");
        }

        let asset_id = vault.allocations.get(0).unwrap().asset_id.clone();
        Self::unlock_tokens_for_asset(env, vault_id, asset_id, amount);
    }

    /// Claims tokens by lender for a specific asset
    pub fn claim_by_lender_for_asset(
        env: Env,
        vault_id: u64,
        lender: Address,
        asset_id: Address,
        amount: i128,
    ) -> i128 {
        let bridge: Address = env
            .storage()
            .instance()
            .get(&DataKey::CollateralBridge)
            .expect("Collateral bridge not set");
        bridge.require_auth();

        let mut vault = Self::get_vault_internal(&env, vault_id);

        // Find the asset allocation
        let mut found = false;
        for (i, allocation) in vault.allocations.iter().enumerate() {
            if allocation.asset_id == asset_id {
                if amount > allocation.locked_amount {
                    panic!("Cannot claim more than locked amount");
                }

                let mut updated_allocation = allocation.clone();
                updated_allocation.locked_amount -= amount;
                vault.allocations.set(i.try_into().unwrap(), updated_allocation);
                found = true;
                break;
            }
        }

        if !found {
            panic!("Asset not found in vault");
        }

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        token::Client::new(&env, &asset_id)
            .transfer(&env.current_contract_address(), &lender, &amount);

        amount
    }
    /// Gets the asset basket for a vault
    pub fn get_vault_asset_basket(env: Env, vault_id: u64) -> Vec<AssetAllocation> {
        let vault = Self::get_vault_internal(&env, vault_id);
        vault.allocations
    }

    /// Updates the asset basket for a vault (admin only, before initialization)
    pub fn update_vault_asset_basket(env: Env, vault_id: u64, new_basket: Vec<AssetAllocation>) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);

        if vault.is_initialized {
            panic!("Cannot update asset basket after initialization");
        }

        if !Self::validate_asset_basket(&new_basket) {
            panic!("Asset basket percentages must sum to 10000 (100%)");
        }

        if new_basket.is_empty() {
            panic!("Asset basket cannot be empty");
        }

        vault.allocations = new_basket;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    /// Gets vault statistics for diversified vesting
    pub fn get_vault_statistics(env: Env, vault_id: u64) -> (i128, i128, i128, u32) {
        let vault = Self::get_vault_internal(&env, vault_id);
        let total_value = Self::calculate_basket_total_value(&vault.allocations);
        let released_value = Self::calculate_basket_released_value(&vault.allocations);
        let claimable_value = Self::calculate_claimable(&env, vault_id, &vault) - released_value;
        let asset_count = vault.allocations.len() as u32;

        (total_value, released_value, claimable_value, asset_count)
    }

    /// Legacy function for single-asset vaults
    pub fn claim_by_lender(env: Env, vault_id: u64, lender: Address, amount: i128) -> i128 {
        let vault = Self::get_vault_internal(&env, vault_id);
        if vault.allocations.len() != 1 {
            panic!("Use claim_by_lender_for_asset for multi-asset vaults");
        }

        let asset_id = vault.allocations.get(0).unwrap().asset_id.clone();
        Self::claim_by_lender_for_asset(env, vault_id, lender, asset_id, amount)
    }

    pub fn set_collateral_bridge(env: Env, bridge_address: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::CollateralBridge, &bridge_address);
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::AdminAddress).expect("Admin not set")
    }

    pub fn get_vault(env: Env, vault_id: u64) -> Vault {
        Self::get_vault_internal(&env, vault_id)
    }


    pub fn set_metadata_anchor(env: Env, cid: String) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::MetadataAnchor, &cid);
    }

    pub fn get_metadata_anchor(env: Env) -> String {
        env.storage().instance().get(&DataKey::MetadataAnchor)
            .unwrap_or(String::from_str(&env, ""))
    }
    pub fn get_user_vaults(env: Env, user: Address) -> Vec<u64> {
        env.storage().instance().get(&DataKey::UserVaults(user)).unwrap_or(Vec::new(&env))
    }

    pub fn get_voting_power(env: Env, user: Address) -> i128 {
        // If this user has delegated their power to someone else, they have 0
        if env.storage().instance().has(&DataKey::VotingDelegate(user.clone())) {
            return 0;
        }

        let mut total_power = Self::calculate_user_own_power(&env, &user);
        
        // Add power from others who delegated to this user
        let delegators: Vec<Address> = env.storage().instance().get(&DataKey::DelegatedBeneficiaries(user)).unwrap_or(vec![&env]);
        for delegator in delegators.iter() {
            total_power += Self::calculate_user_own_power(&env, &delegator);
        }
        
        total_power
    }

    pub fn delegate_voting_power(env: Env, beneficiary: Address, representative: Address) {
        beneficiary.require_auth();
        
        // 1. Get current representative if any
        let old_representative: Option<Address> = env.storage().instance().get(&DataKey::VotingDelegate(beneficiary.clone()));
        
        // 2. If same as before, do nothing
        if let Some(ref old) = old_representative {
            if old == &representative {
                return;
            }
            
            // Remove from old representative's list
            let mut old_list: Vec<Address> = env.storage().instance().get(&DataKey::DelegatedBeneficiaries(old.clone())).unwrap_or(vec![&env]);
            if let Some(idx) = old_list.first_index_of(&beneficiary) {
                old_list.remove(idx);
                env.storage().instance().set(&DataKey::DelegatedBeneficiaries(old.clone()), &old_list);
            }
        }
        
        // 3. Update to new representative
        // If representative is beneficiary itself, it means undelegate
        if beneficiary == representative {
            env.storage().instance().remove(&DataKey::VotingDelegate(beneficiary.clone()));
        } else {
            env.storage().instance().set(&DataKey::VotingDelegate(beneficiary.clone()), &representative);
            
            // Add to new representative's list
            let mut new_list: Vec<Address> = env.storage().instance().get(&DataKey::DelegatedBeneficiaries(representative.clone())).unwrap_or(vec![&env]);
            if !new_list.contains(&beneficiary) {
                new_list.push_back(beneficiary.clone());
                env.storage().instance().set(&DataKey::DelegatedBeneficiaries(representative), &new_list);
            }
        }
    }

    pub fn accelerate_all_schedules(env: Env, percentage: u32) {
        Self::require_admin(&env);
        if percentage > 100 { panic!("Percentage must be between 0 and 100"); }
        env.storage().instance().set(&DataKey::GlobalAccelerationPct, &percentage);
    }

    pub fn slash_unvested_balance(env: Env, vault_id: u64, treasury: Address) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        
        let vested = Self::calculate_claimable(&env, vault_id, &vault);
        let unvested = vault.total_amount - vested;
        
        if unvested <= 0 { panic!("Nothing to slash"); }
        
        // The slashed tokens are taken from the vault's total capacity
        vault.total_amount = vested;
        // Effectively stop the clock for this vault
        vault.end_time = env.ledger().timestamp();
        vault.step_duration = 0;
        
        // Reset milestones to prevent future unlocks from a reduced total
        if env.storage().instance().has(&DataKey::VaultMilestones(vault_id)) {
            env.storage().instance().remove(&DataKey::VaultMilestones(vault_id));
        }
        
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
        
        // Update global tracking
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - unvested));
        
        // Transfer to community treasury
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
        token::Client::new(&env, &token).transfer(&env.current_contract_address(), &treasury, &unvested);
        
        // Emit event
        env.events().publish((Symbol::new(&env, "slash"), vault_id), (vested, unvested, treasury));
    }

    // --- Auto-Stake Functions ---

    /// Whitelist a staking contract address so vaults can stake against it.
    /// Only callable by the admin.
    pub fn add_staking_contract(env: Env, staking_contract: Address) {
        Self::require_admin(&env);
        add_approved_staking_contract(&env, staking_contract);
    }

    /// Remove a staking contract from the whitelist.
    /// Only callable by the admin.
    pub fn remove_staking_contract(env: Env, staking_contract: Address) {
        Self::require_admin(&env);
        remove_approved_staking_contract(&env, staking_contract);
    }

    /// Return the list of whitelisted staking contracts.
    pub fn get_staking_contracts(env: Env) -> Vec<Address> {
        get_approved_staking_contracts(&env)
    }

    /// Register the vault's locked balance as an active stake on `staking_contract`.
    ///
    /// No tokens are transferred — the staking contract records the stake by
    /// trust. The vault's `staked_amount` field is updated to reflect the
    /// registered amount.
    ///
    /// # Panics
    /// - If the vault is frozen or not initialized.
    /// - If the vault is already staked (`AlreadyStaked`).
    /// - If the locked balance is zero (`InsufficientBalance`).
    /// - If `staking_contract` is not whitelisted (`UnauthorizedStakingContract`).
    /// - If the caller is neither the vault owner nor the admin.
    pub fn auto_stake(env: Env, vault_id: u64, staking_contract: Address) {
        Self::require_not_paused(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        if vault.is_frozen { panic!("Vault frozen"); }
        if !vault.is_initialized { panic!("Vault not initialized"); }

        // Auth: owner or admin — require owner auth (admin can mock_all_auths in tests)
        vault.owner.require_auth();

        // Heartbeat: reset Dead-Man's Switch
        update_activity(&env, vault_id);

        // Validate staking contract is whitelisted
        if !is_approved_staking_contract(&env, &staking_contract) {
            panic!("UnauthorizedStakingContract");
        }

        let mut stake_info = get_stake_info(&env, vault_id);

        // Cannot double-stake
        if stake_info.stake_state != StakeState::Unstaked {
            panic!("AlreadyStaked");
        }

        // Must have locked balance
        let locked = vault.total_amount - vault.released_amount;
        if locked <= 0 {
            panic!("InsufficientBalance");
        }

        // Call the staking contract synchronously (Soroban: no async, direct call)
        call_stake_tokens(&env, &staking_contract, &vault.owner, vault_id, locked);

        // Update vault staked_amount
        vault.staked_amount = locked;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        // Update stake info
        stake_info.tokens_staked = locked;
        stake_info.stake_state = StakeState::Staked(env.ledger().timestamp(), staking_contract.clone());
        set_stake_info(&env, vault_id, &stake_info);

        // Update global staked counter
        let total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalStaked, &(total_staked + locked));

        stake::emit_staked(&env, vault_id, &vault.owner, locked, &staking_contract);
    }

    /// Manually unstake a vault. The beneficiary (owner) or admin can call this.
    ///
    /// # Panics
    /// - If the vault is not currently staked (`NotStaked`).
    pub fn manual_unstake(env: Env, vault_id: u64) {
        Self::require_not_paused(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        vault.owner.require_auth();
        // Heartbeat: reset Dead-Man's Switch
        update_activity(&env, vault_id);
        Self::do_unstake(&env, vault_id, &mut vault);
    }

    /// Claim yield accrued on the staking contract for a vault.
    ///
    /// The yield is transferred from the staking contract to the beneficiary.
    /// The vault's `accumulated_yield` is reset to zero after the transfer.
    ///
    /// # Panics
    /// - If the vault is not currently staked (`NotStaked`).
    /// - If the vault has been revoked (`BeneficiaryRevoked`).
    pub fn claim_yield(env: Env, vault_id: u64) -> i128 {
        Self::require_not_paused(&env);
        let vault = Self::get_vault_internal(&env, vault_id);
        vault.owner.require_auth();

        // Heartbeat: reset Dead-Man's Switch
        update_activity(&env, vault_id);

        // Guard: revoked vaults cannot claim yield
        if Self::is_vault_revoked(&env, vault_id) {
            panic!("BeneficiaryRevoked");
        }

        let mut stake_info = get_stake_info(&env, vault_id);

        let staking_contract = match &stake_info.stake_state {
            StakeState::Staked(_, staking_contract) => staking_contract.clone(),
            StakeState::Unstaked => panic!("NotStaked"),
        };

        let yield_amount = call_claim_yield_for(&env, &staking_contract, &vault.owner, vault_id);

        if yield_amount > 0 {
            // Transfer yield from staking contract to beneficiary
            let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
            token::Client::new(&env, &token).transfer(&staking_contract, &vault.owner, &yield_amount);
        }

        stake_info.accumulated_yield = 0;
        set_stake_info(&env, vault_id, &stake_info);

        stake::emit_yield_claimed(&env, vault_id, &vault.owner, yield_amount);
        yield_amount
    }

    /// Revoke a vault: slash all unvested tokens to `treasury`.
    ///
    /// If the vault is currently staked, it is automatically unstaked first
    /// before the treasury transfer. This ensures tokens are never stuck in a
    /// staked state after revocation.
    ///
    /// # Panics
    /// - If the vault is marked irrevocable.
    pub fn revoke_vault(env: Env, vault_id: u64, treasury: Address) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);

        if vault.is_irrevocable {
            panic!("Vault is irrevocable");
        }

        // Auto-unstake if staked
        let stake_info = get_stake_info(&env, vault_id);
        if stake_info.stake_state != StakeState::Unstaked {
            Self::do_unstake(&env, vault_id, &mut vault);
            stake::emit_revocation_unstaked(&env, vault_id, &vault.owner);
        }

        // Mark vault as revoked
        Self::mark_vault_revoked(&env, vault_id);

        // Slash all remaining tokens to treasury
        let remaining = vault.total_amount - vault.released_amount;
        if remaining > 0 {
            vault.total_amount = vault.released_amount;
            vault.end_time = env.ledger().timestamp();
            vault.step_duration = 0;
            vault.is_frozen = true;

            if env.storage().instance().has(&DataKey::VaultMilestones(vault_id)) {
                env.storage().instance().remove(&DataKey::VaultMilestones(vault_id));
            }

            env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

            let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
            env.storage().instance().set(&DataKey::TotalShares, &(total_shares - remaining));

            let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
            token::Client::new(&env, &token).transfer(&env.current_contract_address(), &treasury, &remaining);

            env.events().publish(
                (Symbol::new(&env, "revoked"), vault_id),
                (vault.owner, remaining, treasury),
            );
        }
    }

    /// Return the current stake status for a vault.
    pub fn get_stake_status(env: Env, vault_id: u64) -> StakeStatusView {
        let info = get_stake_info(&env, vault_id);
        StakeStatusView {
            vault_id,
            stake_state: info.stake_state,
            tokens_staked: info.tokens_staked,
            accumulated_yield: info.accumulated_yield,
        }
    }

    // --- Inheritance / Dead-Man's Switch Functions ---

    /// Nominate a backup address and configure the inactivity timer.
    ///
    /// # Security
    /// - Caller must be the vault's current primary beneficiary.
    /// - `backup` must not equal the primary and must not be the zero address.
    /// - `switch_duration` must be within `[MIN_SWITCH_DURATION, MAX_SWITCH_DURATION]`.
    /// - `challenge_window` must be within `[MIN_CHALLENGE_WINDOW, MAX_CHALLENGE_WINDOW]`.
    /// - Cannot be called after succession has been finalised.
    pub fn nominate_backup(
        env: Env,
        vault_id: u64,
        backup: Address,
        switch_duration: u64,
        challenge_window: u64,
    ) {
        let vault = Self::get_vault_internal(&env, vault_id);
        nominate_backup(&env, vault_id, &vault.owner, backup, switch_duration, challenge_window);
    }

    /// Revoke the nominated backup, resetting succession state to `None`.
    ///
    /// # Security
    /// - Caller must be the vault's current primary beneficiary.
    /// - Only valid when state is `Nominated` — blocked during an active claim.
    pub fn revoke_backup(env: Env, vault_id: u64) {
        let vault = Self::get_vault_internal(&env, vault_id);
        revoke_backup(&env, vault_id, &vault.owner);
    }

    /// Initiate a succession claim as the nominated backup.
    ///
    /// # Security
    /// - Caller must be the nominated backup address.
    /// - The inactivity timer must have fully elapsed.
    pub fn initiate_succession_claim(env: Env, vault_id: u64, caller: Address) {
        initiate_succession_claim(&env, vault_id, &caller);
    }

    /// Finalise succession, permanently transferring vault ownership to the backup.
    ///
    /// # Security
    /// - Caller must be the backup address.
    /// - The challenge window must have fully elapsed.
    /// - This operation is irreversible.
    pub fn finalise_succession(env: Env, vault_id: u64, caller: Address) {
        let new_owner = finalise_succession(&env, vault_id, &caller);
        // Update the vault's owner field to the new primary
        let mut vault = Self::get_vault_internal(&env, vault_id);
        vault.owner = new_owner;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    /// Cancel a pending succession claim. Resets state to `Nominated`.
    ///
    /// # Security
    /// - Caller must be the current primary beneficiary.
    /// - State must be `ClaimPending`.
    pub fn cancel_succession_claim(env: Env, vault_id: u64) {
        let vault = Self::get_vault_internal(&env, vault_id);
        cancel_succession_claim(&env, vault_id, &vault.owner);
    }

    /// Return the full succession status for a vault.
    pub fn get_succession_status(env: Env, vault_id: u64) -> SuccessionView {
        let vault = Self::get_vault_internal(&env, vault_id);
        get_succession_status(&env, vault_id, vault.owner)
    }

    // --- Internal Helpers ---

    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::AdminAddress)
            .expect("Admin not set");
        admin.require_auth();
    }

    fn require_pause_authority(env: &Env) {
        // Check if there's a designated pause authority
        if
            let Some(authority) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::PauseAuthority)
        {
            authority.require_auth();
        } else {
            // Fallback to admin if no specific pause authority is set
            Self::require_admin(env);
        }
    }

    fn require_not_paused(env: &Env) {
        if env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false) {
            panic!("Paused");
        }
    }

    fn require_collateral_bridge(env: &Env) {
        let bridge: Address = env
            .storage()
            .instance()
            .get(&DataKey::CollateralBridge)
            .expect("Collateral bridge not set");
        bridge.require_auth();
    }

    fn require_valid_duration(start: u64, end: u64) {
        if end <= start {
            panic!("Invalid duration");
        }
        if end - start > MAX_DURATION {
            panic!("duration exceeds MAX_DURATION");
        }
    }

    fn create_vault_full_internal(
        env: &Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64
    ) -> u64 {
        // For backward compatibility, create a single-asset vault
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
        let allocation = AssetAllocation {
            asset_id: token,
            total_amount: amount,
            released_amount: 0,
            locked_amount: 0,
            percentage: 10000, // 100% in basis points
        };
        let mut allocations = Vec::new(env);
        allocations.push_back(allocation);
        
        Self::sub_admin_balance(env, amount);
        Self::create_vault_prefunded_internal(
            env,
            owner,
            allocations,
            start_time,
            end_time,
            keeper_fee,
            is_revocable,
            is_transferable,
            step_duration,
            true,
        )
    }

    fn create_vault_lazy_internal(
        env: &Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64
    ) -> u64 {
        // For backward compatibility, create a single-asset vault
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
        let allocation = AssetAllocation {
            asset_id: token,
            total_amount: amount,
            released_amount: 0,
            locked_amount: 0,
            percentage: 10000, // 100% in basis points
        };
        let mut allocations = Vec::new(env);
        allocations.push_back(allocation);
        
        Self::create_vault_prefunded_internal(
            env,
            owner,
            allocations,
            start_time,
            end_time,
            keeper_fee,
            is_revocable,
            is_transferable,
            step_duration,
            false,
        )
    }

    fn create_vault_prefunded_internal(
        env: &Env, 
        owner: Address, 
        allocations: Vec<AssetAllocation>, 
        start_time: u64, 
        end_time: u64,
        keeper_fee: i128, 
        is_revocable: bool, 
        is_transferable: bool, 
        step_duration: u64,
        is_initialized: bool,
    ) -> u64 {
        Self::require_valid_duration(start_time, end_time);
        let id = Self::increment_vault_count(env);
        let vault = Vault {
            allocations,
            keeper_fee,
            staked_amount: 0,
            owner: owner.clone(),
            delegate: None,
            title: String::from_str(env, ""),
            start_time,
            end_time,
            creation_time: env.ledger().timestamp(),
            step_duration,
            is_initialized,
            is_irrevocable: !is_revocable,
            is_transferable,
            is_frozen: false,
        };
        env.storage().instance().set(&DataKey::VaultData(id), &vault);
        if is_initialized {
            Self::add_user_vault_index(env, &owner, id);
        }
        let total_amount = Self::calculate_basket_total_value(&vault.allocations);
        Self::add_total_shares(env, total_amount);
        id
    }

    fn get_vault_internal(env: &Env, id: u64) -> Vault {
        env.storage().instance().get(&DataKey::VaultData(id)).expect("Vault not found")
    }

    fn increment_vault_count(env: &Env) -> u64 {
        let count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&DataKey::VaultCount, &new_count);
        new_count
    }

    fn sub_admin_balance(env: &Env, amount: i128) {
        let bal: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if bal < amount {
            panic!("Insufficient admin balance");
        }
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &(bal - amount));
    }

    fn reserve_admin_balance_for_batch(env: &Env, amount: i128) {
        let bal: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if bal < amount { panic!("Insufficient admin balance for batch"); }
        env.storage().instance().set(&DataKey::AdminBalance, &(bal - amount));
    }

    fn add_total_shares(env: &Env, amount: i128) {
        let shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalShares, &(shares + amount));
    }

    fn require_deposited_tokens_for_batch(env: &Env, amount: i128) {
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
        let contract_address = env.current_contract_address();
        let onchain_balance = token::Client::new(env, &token).balance(&contract_address);
        if onchain_balance < amount {
            panic!("Insufficient deposited tokens for batch");
        }
    }

    fn validate_batch_data(data: &BatchCreateData) -> i128 {
        let count = data.recipients.len();
        if count == 0 {
            panic!("Empty batch");
        }
        if data.amounts.len() != count
            || data.start_times.len() != count
            || data.end_times.len() != count
            || data.keeper_fees.len() != count
            || !(data.step_durations.len() == count || data.step_durations.is_empty())
        {
            panic!("Invalid batch data");
        }

        let mut total_amount: i128 = 0;
        for i in 0..count {
            let amount = data.amounts.get(i).unwrap();
            if amount < 0 {
                panic!("Invalid amount");
            }

            let start_time = data.start_times.get(i).unwrap();
            let end_time = data.end_times.get(i).unwrap();
            Self::require_valid_duration(start_time, end_time);

            total_amount = total_amount
                .checked_add(amount)
                .expect("Batch amount overflow");
        }
        total_amount
    }

    fn validate_schedule_configs(schedules: &Vec<ScheduleConfig>) -> i128 {
        if schedules.is_empty() {
            panic!("Empty batch");
        }

        let mut total_amount: i128 = 0;
        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if schedule.amount < 0 {
                panic!("Invalid amount");
            }

            Self::require_valid_duration(schedule.start_time, schedule.end_time);
            total_amount = total_amount
                .checked_add(schedule.amount)
                .expect("Batch amount overflow");
        }
        total_amount
    }

    fn add_user_vault_index(env: &Env, user: &Address, id: u64) {
        let mut vaults: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(user.clone()))
            .unwrap_or(vec![env]);
        vaults.push_back(id);
        env.storage().instance().set(&DataKey::UserVaults(user.clone()), &vaults);
    }

    fn calculate_user_own_power(env: &Env, user: &Address) -> i128 {
        let vault_ids = env.storage().instance().get(&DataKey::UserVaults(user.clone())).unwrap_or(vec![env]);
        let mut total_power: i128 = 0;
        for id in vault_ids.iter() {
            let vault = Self::get_vault_internal(env, id);
            let balance = vault.total_amount - vault.released_amount;
            let weight = if vault.is_irrevocable { 100 } else { 50 };
            total_power += (balance * weight) / 100;
        }
        total_power
    }

    /// Internal: perform the unstake operation against the staking contract and
    /// update vault + stake_info state. Caller must have already loaded `vault`.
    fn do_unstake(env: &Env, vault_id: u64, vault: &mut crate::Vault) {
        let mut stake_info = get_stake_info(env, vault_id);

        let staking_contract = match &stake_info.stake_state {
            StakeState::Staked(_, staking_contract) => staking_contract.clone(),
            StakeState::Unstaked => panic!("NotStaked"),
        };

        call_unstake_tokens(env, &staking_contract, &vault.owner, vault_id);

        // Update global staked counter
        let total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
        let new_total = if total_staked > stake_info.tokens_staked {
            total_staked - stake_info.tokens_staked
        } else {
            0
        };
        env.storage().instance().set(&DataKey::TotalStaked, &new_total);

        let unstaked_amount = stake_info.tokens_staked;
        vault.staked_amount = 0;
        env.storage().instance().set(&DataKey::VaultData(vault_id), vault);

        stake_info.tokens_staked = 0;
        stake_info.stake_state = StakeState::Unstaked;
        set_stake_info(env, vault_id, &stake_info);

        stake::emit_unstaked(env, vault_id, &vault.owner, unstaked_amount);
    }

    /// Mark a vault as revoked in the global revoked-vaults set.
    fn mark_vault_revoked(env: &Env, vault_id: u64) {
        let mut revoked: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::RevokedVaults)
            .unwrap_or(Vec::new(env));
        if !revoked.contains(&vault_id) {
            revoked.push_back(vault_id);
            env.storage().instance().set(&DataKey::RevokedVaults, &revoked);
        }
    }

    /// Returns `true` if the vault has been revoked.
    fn is_vault_revoked(env: &Env, vault_id: u64) -> bool {
        let revoked: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::RevokedVaults)
            .unwrap_or(Vec::new(env));
        revoked.contains(&vault_id)
    }    /// Validates that asset basket percentages sum to 10000 (100%)
    fn validate_asset_basket(basket: &Vec<AssetAllocation>) -> bool {
        let total_percentage: u32 = basket.iter().map(|a| a.percentage).sum();
        total_percentage == 10000 // 100% in basis points
    }

    /// Calculates the total value of all assets in a basket
    fn calculate_basket_total_value(basket: &Vec<AssetAllocation>) -> i128 {
        basket.iter().map(|a| a.total_amount).sum()
    }

    /// Calculates the total released value of all assets in a basket
    fn calculate_basket_released_value(basket: &Vec<AssetAllocation>) -> i128 {
        basket.iter().map(|a| a.released_amount).sum()
    }

    /// Creates a new asset allocation with validation
    pub fn create_asset_allocation(
        asset_id: Address,
        total_amount: i128,
        percentage: u32,
    ) -> AssetAllocation {
        if total_amount <= 0 {
            panic!("Asset amount must be positive");
        }
        if percentage == 0 || percentage > 10000 {
            panic!("Asset percentage must be between 1 and 10000 basis points");
        }
        
        AssetAllocation {
            asset_id,
            total_amount,
            released_amount: 0,
            locked_amount: 0,
            percentage,
        }
    }

    /// Calculates claimable amount for a specific asset in the basket
    fn calculate_claimable_for_asset(env: &Env, id: u64, vault: &Vault, asset_index: usize) -> i128 {
        let allocation = vault.allocations.get(asset_index.try_into().unwrap()).unwrap();
        
        // Check if performance cliff is set and if it's passed
        if let Some(cliff) = env.storage().instance().get(&DataKey::VaultPerformanceCliff(id)) {
            if !OracleClient::is_cliff_passed(env, &cliff, id) {
                // Cliff not passed, no vesting yet
                return 0;
            }
        }

        // If vault is paused, calculate based on pause timestamp
        if let Some(paused_info) = env
            .storage()
            .instance()
            .get::<DataKey, PausedVault>(&DataKey::PausedVault(id))
        {
            let pause_time = paused_info.pause_timestamp;
            if pause_time <= vault.start_time {
                return 0;
            }
            if pause_time >= vault.end_time {
                return allocation.total_amount;
            }

            let duration = (vault.end_time - vault.start_time) as i128;
            let elapsed = (pause_time - vault.start_time) as i128;

            if vault.step_duration > 0 {
                let steps = duration / (vault.step_duration as i128);
                if steps == 0 { return 0; }
                let completed = elapsed / (vault.step_duration as i128);
                (allocation.total_amount * completed) / steps
            } else {
                (allocation.total_amount * elapsed) / duration
            }
        } else if env.storage().instance().has(&DataKey::VaultMilestones(id)) {
            let milestones: Vec<Milestone> = env
                .storage()
                .instance()
                .get(&DataKey::VaultMilestones(id))
                .expect("No milestones");
            let mut pct = 0;
            for m in milestones.iter() {
                if m.is_unlocked {
                    pct += m.percentage;
                }
            }
            if pct > 100 {
                pct = 100;
            }
            (allocation.total_amount * (pct as i128)) / 100
        } else {
            let mut now = env.ledger().timestamp();
            let accel_pct: u32 = env.storage().instance().get(&DataKey::GlobalAccelerationPct).unwrap_or(0);
            
            let duration = (vault.end_time - vault.start_time) as i128;
            if accel_pct > 0 {
                let shift = (duration * accel_pct as i128 / 100) as u64;
                now += shift;
            }

            if now <= vault.start_time { return 0; }
            if now >= vault.end_time { return allocation.total_amount; }
            
            let elapsed = (now - vault.start_time) as i128;

            if vault.step_duration > 0 {
                let steps = duration / vault.step_duration as i128;
                if steps == 0 { return 0; }
                let completed = elapsed / vault.step_duration as i128;
                (allocation.total_amount * completed) / steps
            } else {
                (allocation.total_amount * elapsed) / duration
            }
        }
    }

    /// Legacy function for backward compatibility - calculates total claimable across all assets
    fn calculate_claimable(env: &Env, id: u64, vault: &Vault) -> i128 {
        let mut total_claimable = 0;
        for i in 0..vault.allocations.len() {
            total_claimable += Self::calculate_claimable_for_asset(env, id, vault, i.try_into().unwrap());
        }
        total_claimable
    // --- Governance Helper Functions ---

    fn create_governance_proposal(env: Env, action: GovernanceAction) -> u64 {
        let proposer = Self::get_admin(&env);
        let now = env.ledger().timestamp();
        let proposal_id = Self::increment_proposal_count(&env);
        
        let proposal = GovernanceProposal {
            id: proposal_id,
            action: action.clone(),
            proposer: proposer.clone(),
            created_at: now,
            challenge_end: now + CHALLENGE_PERIOD,
            is_executed: false,
            is_cancelled: false,
            yes_votes: 0,
            no_votes: 0,
        };

        env.storage().instance().set(&DataKey::GovernanceProposal(proposal_id), &proposal);

        // Publish proposal creation event
        let proposal_event = GovernanceProposalCreated {
            proposal_id,
            action: action.clone(),
            proposer,
            challenge_end: proposal.challenge_end,
        };
        env.events().publish((Symbol::new(&env, "governance_proposal"), proposal_id), proposal_event);

        proposal_id
    }

    fn get_proposal(env: &Env, proposal_id: u64) -> GovernanceProposal {
        env.storage().instance()
            .get(&DataKey::GovernanceProposal(proposal_id))
            .expect("Proposal not found")
    }

    fn get_voter_locked_value(env: &Env, voter: &Address) -> i128 {
        // Get all vaults for this voter and sum their total amounts
        let vault_ids: Vec<u64> = env.storage().instance()
            .get(&DataKey::UserVaults(voter.clone()))
            .unwrap_or(Vec::new(env));
        
        let mut total_locked = 0i128;
        for vault_id in vault_ids.iter() {
            let vault = Self::get_vault_internal(env, *vault_id);
            total_locked += vault.total_amount - vault.released_amount;
        }
        
        total_locked
    }

    fn get_total_locked_value(env: &Env) -> i128 {
        env.storage().instance()
            .get(&DataKey::TotalLockedValue)
            .unwrap_or(0i128)
    }

    fn execute_governance_action(env: &Env, action: &GovernanceAction) {
        match action {
            GovernanceAction::AdminRotation(new_admin) => {
                env.storage().instance().set(&DataKey::AdminAddress, new_admin);
            },
            GovernanceAction::ContractUpgrade(new_contract) => {
                env.storage().instance().set(&DataKey::MigrationTarget, new_contract);
                env.storage().instance().set(&DataKey::IsDeprecated, &true);
            },
            GovernanceAction::EmergencyPause(pause_state) => {
                env.storage().instance().set(&DataKey::IsPaused, pause_state);
            },
        }
    }

    fn increment_proposal_count(env: &Env) -> u64 {
        let count: u64 = env.storage().instance().get(&DataKey::ProposalCount).unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&DataKey::ProposalCount, &new_count);
        new_count
    }

    // Public getter functions for governance
    pub fn get_proposal_info(env: Env, proposal_id: u64) -> GovernanceProposal {
        Self::get_proposal(&env, proposal_id)
    }

    pub fn get_voter_power(env: Env, voter: Address) -> i128 {
        Self::get_voter_locked_value(&env, &voter)
    }

    pub fn get_total_locked(env: Env) -> i128 {
        Self::get_total_locked_value(&env)
    }
}

pub mod diversified_core;

#[cfg(test)]
mod test;
#[cfg(test)]
mod invariant_test;
#[cfg(test)]
mod diversified_test;
#[cfg(test)]
mod diversified_simple_test;
