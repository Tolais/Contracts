#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Address, Vec, Map, String};

mod storage;
mod types;
mod audit_exporter;
mod emergency;

use types::{ClaimEvent, AuthorizedAddressSet, AddressWhitelistRequest, AuthorizedPayoutAddress, MilestoneConfig, MilestoneStatus, MilestoneCompleted, ClaimSimulation, ReputationBonus, ReputationBonusApplied, Nullifier, Commitment, ZKClaimProof, PrivacyClaimEvent, CommitmentCreated, PrivateClaimExecuted, PathPaymentConfig, PathPaymentClaimEvent, PathPaymentSimulation};
use storage::{get_claim_history, set_claim_history, get_authorized_payout_address as storage_get_authorized_payout_address, set_authorized_payout_address as storage_set_authorized_payout_address, get_pending_address_request as storage_get_pending_address_request, set_pending_address_request as storage_set_pending_address_request, remove_pending_address_request as storage_remove_pending_address_request, get_timelock_duration, get_auditors, set_auditors, get_auditor_pause_requests, set_auditor_pause_requests, get_emergency_pause, set_emergency_pause, remove_emergency_pause, get_reputation_bridge_contract, set_reputation_bridge_contract, has_reputation_bonus_applied, set_reputation_bonus_applied, get_milestone_configs, set_milestone_configs, get_milestone_status, set_milestone_status, get_emergency_pause_duration, is_nullifier_used, set_nullifier_used, get_commitment, set_commitment, mark_commitment_used, add_privacy_claim_event, add_merkle_root, get_merkle_roots, is_valid_merkle_root, get_path_payment_config, set_path_payment_config, get_path_payment_claim_history, add_path_payment_claim_event};
use emergency::{AuditorPauseRequest, EmergencyPause, EmergencyPauseTriggered, EmergencyPauseLifted};

#[contract]
pub struct VestingVault;

#[contractimpl]
impl VestingVault {

    pub fn claim(e: Env, user: Address, vesting_id: u32, amount: i128) {
        user.require_auth();

        // Check if contract is under emergency pause
        if let Some(pause) = get_emergency_pause(&e) {
            if pause.is_active {
                let current_time = e.ledger().timestamp();
                if current_time < pause.expires_at {
                    panic!("Contract is under emergency pause until {}", pause.expires_at);
                } else {
                    // Pause expired, remove it
                    remove_emergency_pause(&e);
                }
            }
        }

        // Check if user has an authorized payout address
        if let Some(auth_address) = storage_get_authorized_payout_address(&e, &user) {
            if auth_address.is_active {
                let current_time = e.ledger().timestamp();
                
                // Check if timelock has passed
                if current_time < auth_address.effective_at {
                    panic!("Authorized payout address is still in timelock period");
                }
                
                // Verify the claim is being made to the authorized address
                // In a real implementation, this would check the destination of the transfer
                // For now, we'll assume the claim function includes a destination parameter
                // or that the user context provides this information
            }
        }

        // Check milestone vesting if applicable
        if let Some(_milestone_configs) = get_milestone_configs(&e, vesting_id) {
            let _milestone_status = get_milestone_status(&e, vesting_id);
            // Additional milestone logic would go here
        }

        // TODO: your vesting logic here

        let mut history = get_claim_history(&e);

        let event = ClaimEvent {
            beneficiary: user.clone(),
            amount,
            timestamp: e.ledger().timestamp(),
            vesting_id,
        };

        history.push_back(event);

        set_claim_history(&e, &history);
    }

    /// Sets an authorized payout address with a 48-hour timelock
    /// This provides multi-layer defense against phishing hacks
    pub fn set_authorized_payout_address(e: Env, beneficiary: Address, authorized_address: Address) {
        beneficiary.require_auth();
        
        let current_time = e.ledger().timestamp();
        let effective_at = current_time + get_timelock_duration();

        // Create the pending request
        let request = AddressWhitelistRequest {
            beneficiary: beneficiary.clone(),
            requested_address: authorized_address.clone(),
            requested_at: current_time,
            effective_at,
        };

        // Store the pending request
        storage_set_pending_address_request(&e, &beneficiary, &request);

        // Emit event
        e.events().publish(
            ("AddressWhitelistRequested", (), ()),
            (beneficiary.clone(), authorized_address.clone(), current_time, effective_at),
        );
    }

    /// Confirms and activates a pending authorized payout address request
    /// Can only be called after the 48-hour timelock period has passed
    pub fn confirm_auth_payout_addr(e: Env, beneficiary: Address) {
        beneficiary.require_auth();
        
        let current_time = e.ledger().timestamp();
        
        // Get the pending request
        let pending_request = storage_get_pending_address_request(&e, &beneficiary)
            .expect("No pending address request found");

        // Check if timelock has passed
        if current_time < pending_request.effective_at {
            panic!("Timelock period has not yet passed");
        }

        // Create the authorized address record
        let auth_address = AuthorizedPayoutAddress {
            beneficiary: beneficiary.clone(),
            authorized_address: pending_request.requested_address.clone(),
            requested_at: pending_request.requested_at,
            effective_at: pending_request.effective_at,
            is_active: true,
        };

        // Store the authorized address
        storage_set_authorized_payout_address(&e, &beneficiary, &auth_address);

        // Remove the pending request
        storage_remove_pending_address_request(&e, &beneficiary);

        // Emit confirmation event
        e.events().publish(
            ("AuthorizedAddressSet", (), ()),
            (beneficiary.clone(), pending_request.requested_address.clone(), pending_request.effective_at),
        );
    }

    /// Gets the current authorized payout address for a beneficiary
    pub fn get_authorized_payout_address(e: Env, beneficiary: Address) -> Option<AuthorizedPayoutAddress> {
        storage_get_authorized_payout_address(&e, &beneficiary)
    }

    /// Gets any pending address request for a beneficiary
    pub fn get_pending_address_request(e: Env, beneficiary: Address) -> Option<AddressWhitelistRequest> {
        storage_get_pending_address_request(&e, &beneficiary)
    }

    /// Removes the authorized payout address (immediate effect)
    /// This allows beneficiaries to disable the whitelisting feature
    pub fn remove_authorized_payout_address(e: Env, beneficiary: Address) {
        beneficiary.require_auth();
        
        // Remove the authorized address
        e.storage().instance().remove(&(storage::AUTHORIZED_PAYOUT_ADDRESS, beneficiary.clone()));
        
        // Also remove any pending request
        storage_remove_pending_address_request(&e, &beneficiary);
    }

    // 🔍 helper getter (needed for exporter)
    pub fn get_all_claims(e: Env) -> Vec<ClaimEvent> {
        get_claim_history(&e)
    }

    // ========== ISSUE #140: Emergency Protocol Pause for Auditors ==========
    
    /// Initialize the auditor security team
    pub fn initialize_auditors(e: Env, admin: Address, auditors: Vec<Address>) {
        admin.require_auth();
        
        // Require exactly 3 auditors for 2-out-of-3 multisig
        if auditors.len() != 3 {
            panic!("Must have exactly 3 auditors");
        }
        
        set_auditors(&e, &auditors);
    }

    /// Request emergency pause by an auditor
    pub fn request_emergency_pause(e: Env, auditor: Address, reason: String) {
        auditor.require_auth();
        
        // Verify caller is an authorized auditor
        let authorized_auditors = get_auditors(&e);
        if !authorized_auditors.contains(&auditor) {
            panic!("Not an authorized auditor");
        }
        
        let current_time = e.ledger().timestamp();
        let mut requests = get_auditor_pause_requests(&e);
        
        // Check if auditor already requested
        if requests.contains_key(auditor.clone()) {
            panic!("Auditor already requested pause");
        }
        
        let request = AuditorPauseRequest {
            auditor: auditor.clone(),
            timestamp: current_time,
            reason: reason.clone(),
        };
        
        requests.set(auditor.clone(), request);
        set_auditor_pause_requests(&e, &requests);
        
        // Check if we have 2-out-of-3 requests
        if requests.len() >= 2 {
            Self::trigger_emergency_pause(&e);
        }
    }

    /// Internal function to trigger emergency pause
    fn trigger_emergency_pause(e: &Env) {
        let requests = get_auditor_pause_requests(e);
        let current_time = e.ledger().timestamp();
        let pause_duration = get_emergency_pause_duration();
        
        let mut auditors = Vec::new(e);
        let mut reason = String::from_str(e, "Emergency pause requested by auditors: ");
        
        for (auditor_addr, _request) in requests.iter() {
            auditors.push_back(auditor_addr);
            // Simple string concatenation - just use the reason directly
            reason = String::from_str(e, "Emergency pause requested by auditors: ");
        }
        
        let pause = EmergencyPause {
            paused_by: auditors.clone(),
            paused_at: current_time,
            expires_at: current_time + pause_duration,
            reason: reason.clone(),
            is_active: true,
        };
        
        set_emergency_pause(e, &pause);
        
        // Clear the requests
        set_auditor_pause_requests(e, &Map::new(e));
        
        // Emit event
        e.events().publish(
            ("EmergencyPauseTriggered", (), ()),
            (auditors.clone(), current_time, current_time + pause_duration, reason.clone()),
        );
    }

    /// Check if contract is currently paused
    pub fn is_emergency_paused(e: Env) -> bool {
        if let Some(pause) = get_emergency_pause(&e) {
            if pause.is_active {
                let current_time = e.ledger().timestamp();
                return current_time < pause.expires_at;
            }
        }
        false
    }

    /// Get current emergency pause status
    pub fn get_emergency_pause_status(e: Env) -> Option<EmergencyPause> {
        get_emergency_pause(&e)
    }

    // ========== ISSUE #137: Vesting Simulate Claim Dry-Run Helper ==========
    
    /// Simulate a claim to show exact amounts without consuming gas
    pub fn simulate_claim(e: Env, user: Address, _vesting_id: u32) -> ClaimSimulation {
        let current_time = e.ledger().timestamp();
        
        // Check if contract is under emergency pause
        if let Some(pause) = get_emergency_pause(&e) {
            if pause.is_active && current_time < pause.expires_at {
                return ClaimSimulation {
                    tokens_to_release: 0,
                    estimated_gas_fee: 0,
                    tax_withholding_amount: 0,
                    net_amount: 0,
                    can_claim: false,
                    reason: String::from_str(&e, "Contract is under emergency pause"),
                };
            }
        }
        
        // Check authorized payout address timelock
        if let Some(auth_address) = storage_get_authorized_payout_address(&e, &user) {
            if auth_address.is_active && current_time < auth_address.effective_at {
                return ClaimSimulation {
                    tokens_to_release: 0,
                    estimated_gas_fee: 0,
                    tax_withholding_amount: 0,
                    net_amount: 0,
                    can_claim: false,
                    reason: String::from_str(&e, "Authorized payout address is still in timelock period"),
                };
            }
        }
        
        // TODO: Calculate actual vesting amounts
        // This is a placeholder - in real implementation, you'd calculate:
        // - tokens_to_release based on vesting schedule
        // - estimated_gas_fee based on current gas prices
        // - tax_withholding_amount based on tax rules
        
        let tokens_to_release = 1000i128; // Placeholder
        let estimated_gas_fee = 50000u64; // Placeholder in stroops
        let tax_withholding_amount = 50i128; // Placeholder
        let net_amount = tokens_to_release - tax_withholding_amount;
        
        ClaimSimulation {
            tokens_to_release,
            estimated_gas_fee,
            tax_withholding_amount,
            net_amount,
            can_claim: true,
            reason: String::from_str(&e, "Claim available"),
        }
    }

    // ========== ISSUE #139: Cross-Project Reputation Bonus Hook ==========
    
    /// Set the reputation bridge contract address
    pub fn set_reputation_bridge(e: Env, admin: Address, bridge_contract: Address) {
        admin.require_auth();
        set_reputation_bridge_contract(&e, &bridge_contract);
    }

    /// Apply reputation bonus based on cross-project success
    pub fn apply_reputation_bonus(e: Env, beneficiary: Address) {
        beneficiary.require_auth();
        
        // Check if bonus already applied
        if has_reputation_bonus_applied(&e, &beneficiary) {
            panic!("Reputation bonus already applied");
        }
        
        // Get reputation bridge contract
        let _bridge_contract = get_reputation_bridge_contract(&e)
            .expect("Reputation bridge contract not set");
        
        // TODO: Call bridge contract to check completion rate
        // For now, assume 100% completion rate
        let completion_rate = 100u32;
        
        if completion_rate >= 100 {
            let cliff_reduction = 1u32; // 1 month reduction
            let current_time = e.ledger().timestamp();
            
            // Mark bonus as applied
            set_reputation_bonus_applied(&e, &beneficiary);
            
            // Emit event
            e.events().publish(
                ("ReputationBonusApplied", (), ()),
                (beneficiary.clone(), cliff_reduction, current_time),
            );
        }
    }

    /// Check if user has reputation bonus applied
    pub fn has_reputation_bonus(e: Env, beneficiary: Address) -> bool {
        has_reputation_bonus_applied(&e, &beneficiary)
    }

    // ========== ISSUE #138: Milestone-Gated Step Vesting ==========
    
    /// Configure milestone vesting for a vesting schedule
    pub fn configure_milestone_vesting(e: Env, admin: Address, vesting_id: u32, milestone_percentages: Vec<u32>) {
        admin.require_auth();
        
        // Validate percentages sum to 100
        let mut total = 0u32;
        for percentage in milestone_percentages.iter() {
            total += percentage;
        }
        
        if total != 100 {
            panic!("Milestone percentages must sum to 100");
        }
        
        let _config = MilestoneConfig {
            vesting_id,
            milestone_percentages: milestone_percentages.clone(),
            total_milestones: milestone_percentages.len() as u32,
        };
        
        set_milestone_configs(&e, vesting_id, &milestone_percentages);
    }

    /// Complete a milestone (admin only)
    pub fn complete_milestone(e: Env, admin: Address, vesting_id: u32, milestone_number: u32) {
        admin.require_auth();
        
        let mut status = get_milestone_status(&e, vesting_id);
        
        // Check if milestone already completed
        if status.contains_key(milestone_number) {
            panic!("Milestone already completed");
        }
        
        // Check sequential completion (milestone N-1 must be completed)
        if milestone_number > 1 {
            if !status.contains_key(milestone_number - 1) {
                panic!("Previous milestone must be completed first");
            }
        }
        
        // Mark milestone as completed
        status.set(milestone_number, true);
        set_milestone_status(&e, vesting_id, &status);
        
        // Emit event
        e.events().publish(
            ("MilestoneCompleted", (), ()),
            (vesting_id, milestone_number, e.ledger().timestamp()),
        );
    }

    /// Get milestone status for a vesting schedule
    pub fn get_milestone_status(e: Env, vesting_id: u32) -> Map<u32, bool> {
        get_milestone_status(&e, vesting_id)
    }

    /// Get milestone configuration for a vesting schedule
    pub fn get_milestone_config(e: Env, vesting_id: u32) -> Option<Vec<u32>> {
        get_milestone_configs(&e, vesting_id)
    }

    // ========== ISSUE #148 & #95: Zero-Knowledge Privacy Claims Foundation ==========
    
    /// Create a commitment for future private claims
    /// This function allows users to create a commitment that can be used for private claims later
    pub fn create_commitment(e: Env, user: Address, vesting_id: u32, amount: i128, commitment_hash: [u8; 32]) {
        user.require_auth();
        
        // Check if commitment already exists
        if get_commitment(&e, &commitment_hash).is_some() {
            panic!("Commitment already exists");
        }
        
        let current_time = e.ledger().timestamp();
        
        // Create the commitment
        let commitment = Commitment {
            hash: commitment_hash,
            created_at: current_time,
            vesting_id,
            amount,
            is_used: false,
        };
        
        // Store the commitment
        set_commitment(&e, &commitment_hash, &commitment);
        
        // Emit event
        e.events().publish(
            ("CommitmentCreated", (), ()),
            (commitment_hash, vesting_id, amount, current_time),
        );
    }
    
    /// Execute a private claim using ZK proof
    /// This function allows users to claim tokens without revealing their identity
    pub fn private_claim(e: Env, zk_proof: ZKClaimProof, nullifier: Nullifier, amount: i128) {
        // No require_auth() - this is a privacy feature
        
        // Check if contract is under emergency pause
        if let Some(pause) = get_emergency_pause(&e) {
            if pause.is_active {
                let current_time = e.ledger().timestamp();
                if current_time < pause.expires_at {
                    panic!("Contract is under emergency pause until {}", pause.expires_at);
                } else {
                    // Pause expired, remove it
                    remove_emergency_pause(&e);
                }
            }
        }
        
        // Check if nullifier has already been used (prevent double-spending)
        if is_nullifier_used(&e, &nullifier) {
            panic!("Nullifier has already been used");
        }
        
        // Verify the commitment exists and is not used
        let commitment = get_commitment(&e, &zk_proof.commitment_hash)
            .expect("Commitment not found");
        
        if commitment.is_used {
            panic!("Commitment has already been used");
        }
        
        // Verify the commitment amount matches the claim amount
        if commitment.amount != amount {
            panic!("Claim amount does not match commitment amount");
        }
        
        // Verify the Merkle root is valid (for ZK proof verification)
        if !is_valid_merkle_root(&e, &zk_proof.merkle_root) {
            panic!("Invalid Merkle root");
        }
        
        // TODO: Verify actual ZK-SNARK proof
        // This is a placeholder for the actual ZK proof verification
        // In a full implementation, this would use a ZK verification library
        Self::verify_zk_proof(&e, &zk_proof);
        
        // Mark nullifier as used
        set_nullifier_used(&e, &nullifier);
        
        // Mark commitment as used
        mark_commitment_used(&e, &zk_proof.commitment_hash);
        
        // Create privacy claim event
        let current_time = e.ledger().timestamp();
        let privacy_event = PrivacyClaimEvent {
            nullifier: nullifier.clone(),
            amount,
            timestamp: current_time,
            vesting_id: commitment.vesting_id,
            is_private: true,
        };
        
        // Add to privacy claim history
        add_privacy_claim_event(&e, &privacy_event);
        
        // Emit event
        e.events().publish(
            ("PrivateClaimExecuted", (), ()),
            (nullifier.hash, amount, current_time),
        );
        
        // TODO: Execute actual token transfer
        // This would integrate with the existing vesting logic
    }
    
    /// Add a Merkle root for ZK proof verification
    /// This function is called by the admin to add new Merkle roots
    pub fn add_merkle_root_admin(e: Env, admin: Address, merkle_root: [u8; 32]) {
        admin.require_auth();
        
        // Check if Merkle root already exists
        if is_valid_merkle_root(&e, &merkle_root) {
            panic!("Merkle root already exists");
        }
        
        // Add the Merkle root
        add_merkle_root(&e, &merkle_root);
    }
    
    /// Get all valid Merkle roots
    pub fn get_merkle_roots(e: Env) -> Vec<[u8; 32]> {
        get_merkle_roots(&e)
    }
    
    /// Check if a nullifier has been used
    pub fn is_nullifier_used_public(e: Env, nullifier: Nullifier) -> bool {
        is_nullifier_used(&e, &nullifier)
    }
    
    /// Get commitment information
    pub fn get_commitment_info(e: Env, commitment_hash: [u8; 32]) -> Option<Commitment> {
        get_commitment(&e, &commitment_hash)
    }
    
    /// Get privacy claim history
    pub fn get_privacy_claim_history(e: Env) -> Vec<PrivacyClaimEvent> {
        storage::get_privacy_claim_history(&e)
    }
    
    /// Placeholder for ZK proof verification
    /// In a full implementation, this would verify the actual ZK-SNARK proof
    fn verify_zk_proof(_e: &Env, _zk_proof: &ZKClaimProof) -> bool {
        // TODO: Implement actual ZK proof verification
        // For now, we'll assume the proof is valid
        // In production, this would integrate with a ZK verification library
        true
    }
    
    /// Enable privacy mode for a vesting schedule
    /// This allows beneficiaries to choose between public and private claims
    pub fn enable_privacy_mode(e: Env, user: Address, vesting_id: u32) {
        user.require_auth();
        
        // TODO: Implement privacy mode toggle
        // This would allow users to enable/disable privacy for their vesting
        // For now, this is a placeholder for the architectural foundation
    }
    
    /// Disable privacy mode for a vesting schedule
    pub fn disable_privacy_mode(e: Env, user: Address, vesting_id: u32) {
        user.require_auth();
        
        // TODO: Implement privacy mode toggle
        // This would allow users to enable/disable privacy for their vesting
        // For now, this is a placeholder for the architectural foundation
    }

    // ========== ISSUE #146 & #93: Stellar Horizon Path Payment Claim ==========
    
    /// Configure path payment settings for auto-exit feature
    /// This allows users to claim tokens and instantly swap them for USDC in one transaction
    pub fn configure_path_payment(e: Env, admin: Address, destination_asset: Address, min_destination_amount: i128, path: Vec<Address>) {
        admin.require_auth();
        
        let config = PathPaymentConfig {
            destination_asset: destination_asset.clone(),
            min_destination_amount,
            path: path.clone(),
            enabled: true,
        };
        
        set_path_payment_config(&e, &config);
        
        // Emit configuration event
        e.events().publish(
            ("PathPaymentConfigured", (), ()),
            (destination_asset, min_destination_amount, path, e.ledger().timestamp()),
        );
    }
    
    /// Disable path payment feature
    pub fn disable_path_payment(e: Env, admin: Address) {
        admin.require_auth();
        
        if let Some(mut config) = get_path_payment_config(&e) {
            config.enabled = false;
            set_path_payment_config(&e, &config);
            
            // Emit disable event
            e.events().publish(
                ("PathPaymentDisabled", (), ()),
                (e.ledger().timestamp(),),
            );
        }
    }
    
    /// Claim tokens with automatic path payment to USDC (Auto-Exit feature)
    /// This allows users to instantly swap their claimed tokens for USDC in one transaction
    pub fn claim_with_path_payment(e: Env, user: Address, vesting_id: u32, amount: i128, min_destination_amount: Option<i128>) {
        user.require_auth();

        // Check if contract is under emergency pause
        if let Some(pause) = get_emergency_pause(&e) {
            if pause.is_active {
                let current_time = e.ledger().timestamp();
                if current_time < pause.expires_at {
                    panic!("Contract is under emergency pause until {}", pause.expires_at);
                } else {
                    // Pause expired, remove it
                    remove_emergency_pause(&e);
                }
            }
        }

        // Get path payment configuration
        let config = get_path_payment_config(&e)
            .expect("Path payment not configured");

        if !config.enabled {
            panic!("Path payment feature is disabled");
        }

        // Use provided min_destination_amount or fallback to config
        let final_min_amount = min_destination_amount.unwrap_or(config.min_destination_amount);
        
        // Validate the amount
        if final_min_amount <= 0 {
            panic!("Minimum destination amount must be positive");
        }

        // TODO: Calculate actual vesting amounts and validate claim
        // This would integrate with the existing vesting logic
        let actual_claimable_amount = amount; // Placeholder - should calculate based on vesting schedule
        
        if actual_claimable_amount <= 0 {
            panic!("No tokens available to claim");
        }

        // Execute the path payment using Stellar's built-in path_payment_strict_receive
        // This is the core of the Auto-Exit feature
        let destination_amount = Self::execute_path_payment(&e, &user, actual_claimable_amount, &config.destination_asset, final_min_amount, &config.path);
        
        // Record the path payment claim event
        let current_time = e.ledger().timestamp();
        let path_payment_event = PathPaymentClaimEvent {
            beneficiary: user.clone(),
            source_amount: actual_claimable_amount,
            destination_amount,
            destination_asset: config.destination_asset.clone(),
            timestamp: current_time,
            vesting_id,
        };
        
        add_path_payment_claim_event(&e, &path_payment_event);
        
        // Also record in regular claim history for compatibility
        let mut history = get_claim_history(&e);
        let claim_event = ClaimEvent {
            beneficiary: user.clone(),
            amount: actual_claimable_amount,
            timestamp: current_time,
            vesting_id,
        };
        history.push_back(claim_event);
        set_claim_history(&e, &history);
        
        // Emit the path payment claim event
        e.events().publish(
            ("PathPaymentClaimExecuted", (), ()),
            (user.clone(), actual_claimable_amount, destination_amount, config.destination_asset.clone(), current_time, vesting_id),
        );
    }
    
    /// Simulate a path payment claim to show expected amounts without consuming gas
    pub fn simulate_path_payment_claim(e: Env, user: Address, vesting_id: u32, amount: i128, min_destination_amount: Option<i128>) -> PathPaymentSimulation {
        let current_time = e.ledger().timestamp();
        
        // Check if contract is under emergency pause
        if let Some(pause) = get_emergency_pause(&e) {
            if pause.is_active && current_time < pause.expires_at {
                return PathPaymentSimulation {
                    source_amount: amount,
                    estimated_destination_amount: 0,
                    min_destination_amount: min_destination_amount.unwrap_or(0),
                    path: Vec::new(&e),
                    can_execute: false,
                    reason: String::from_str(&e, "Contract is under emergency pause"),
                    estimated_gas_fee: 0,
                };
            }
        }
        
        // Check if path payment is configured and enabled
        let config = match get_path_payment_config(&e) {
            Some(c) => c,
            None => {
                return PathPaymentSimulation {
                    source_amount: amount,
                    estimated_destination_amount: 0,
                    min_destination_amount: min_destination_amount.unwrap_or(0),
                    path: Vec::new(&e),
                    can_execute: false,
                    reason: String::from_str(&e, "Path payment not configured"),
                    estimated_gas_fee: 0,
                };
            }
        };
        
        if !config.enabled {
            return PathPaymentSimulation {
                source_amount: amount,
                estimated_destination_amount: 0,
                min_destination_amount: min_destination_amount.unwrap_or(0),
                path: config.path.clone(),
                can_execute: false,
                reason: String::from_str(&e, "Path payment feature is disabled"),
                estimated_gas_fee: 0,
            };
        }
        
        // Use provided min_destination_amount or fallback to config
        let final_min_amount = min_destination_amount.unwrap_or(config.min_destination_amount);
        
        // TODO: Calculate actual vesting amounts
        // This would integrate with the existing vesting logic
        let actual_claimable_amount = amount; // Placeholder
        
        if actual_claimable_amount <= 0 {
            return PathPaymentSimulation {
                source_amount: amount,
                estimated_destination_amount: 0,
                min_destination_amount: final_min_amount,
                path: config.path.clone(),
                can_execute: false,
                reason: String::from_str(&e, "No tokens available to claim"),
                estimated_gas_fee: 0,
            };
        }
        
        // Simulate the path payment (in real implementation, this would query Stellar DEX)
        let estimated_destination_amount = Self::simulate_path_payment_result(&e, actual_claimable_amount, &config.destination_asset, &config.path);
        
        let can_execute = estimated_destination_amount >= final_min_amount;
        
        PathPaymentSimulation {
            source_amount: actual_claimable_amount,
            estimated_destination_amount,
            min_destination_amount: final_min_amount,
            path: config.path.clone(),
            can_execute,
            reason: if can_execute {
                String::from_str(&e, "Path payment claim available")
            } else {
                String::from_str(&e, "Insufficient liquidity for minimum destination amount")
            },
            estimated_gas_fee: 150000u64, // Higher gas fee due to path payment complexity
        }
    }
    
    /// Get current path payment configuration
    pub fn get_path_payment_config(e: Env) -> Option<PathPaymentConfig> {
        get_path_payment_config(&e)
    }
    
    /// Get path payment claim history
    pub fn get_path_payment_claim_history(e: Env) -> Vec<PathPaymentClaimEvent> {
        get_path_payment_claim_history(&e)
    }
    
    /// Internal function to execute the path payment using Stellar's path_payment_strict_receive
    /// This is the core logic that enables the Auto-Exit feature
    fn execute_path_payment(e: &Env, beneficiary: &Address, source_amount: i128, destination_asset: &Address, min_destination_amount: i128, path: &Vec<Address>) -> i128 {
        // In a real Stellar Soroban implementation, this would use the built-in
        // path_payment_strict_receive function from the Stellar SDK
        
        // For this implementation, we simulate the path payment execution
        // In production, this would be:
        // e.invoke_contract::<i128>(
        //     &stellar_sdk::STELLAR_ASSET_CONTRACT,
        //     &symbol_short!("path_payment_strict_receive"),
        //     (beneficiary, source_amount, destination_asset, min_destination_amount, path)
        // );
        
        // Placeholder implementation - simulate successful path payment
        let simulated_destination_amount = Self::simulate_path_payment_result(e, source_amount, destination_asset, path);
        
        if simulated_destination_amount < min_destination_amount {
            panic!("Path payment failed: insufficient liquidity for minimum destination amount");
        }
        
        simulated_destination_amount
    }
    
    /// Internal function to simulate path payment result
    /// In production, this would query the Stellar DEX for real rates
    fn simulate_path_payment_result(_e: &Env, source_amount: i128, _destination_asset: &Address, _path: &Vec<Address>) -> i128 {
        // Placeholder: assume 1:1 conversion rate for simulation
        // In production, this would query the Stellar DEX for actual exchange rates
        // considering the provided path and current market conditions
        
        // For USDC destination, we can assume close to 1:1 with small slippage
        let slippage_factor = 9950; // 99.5% (0.5% slippage)
        (source_amount * slippage_factor) / 10000
    }
}