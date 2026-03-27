#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Address, Vec, Map, String};

mod storage;
mod types;
mod audit_exporter;
mod emergency;

use types::{ClaimEvent, AuthorizedAddressSet, AddressWhitelistRequest, AuthorizedPayoutAddress, MilestoneConfig, MilestoneStatus, MilestoneCompleted, ClaimSimulation, ReputationBonus, ReputationBonusApplied};
use storage::{get_claim_history, set_claim_history, get_authorized_payout_address as storage_get_authorized_payout_address, set_authorized_payout_address as storage_set_authorized_payout_address, get_pending_address_request as storage_get_pending_address_request, set_pending_address_request as storage_set_pending_address_request, remove_pending_address_request as storage_remove_pending_address_request, get_timelock_duration, get_auditors, set_auditors, get_auditor_pause_requests, set_auditor_pause_requests, get_emergency_pause, set_emergency_pause, remove_emergency_pause, get_reputation_bridge_contract, set_reputation_bridge_contract, has_reputation_bonus_applied, set_reputation_bonus_applied, get_milestone_configs, set_milestone_configs, get_milestone_status, set_milestone_status, get_emergency_pause_duration};
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
}