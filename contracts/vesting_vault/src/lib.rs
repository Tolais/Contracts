#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Address, Vec};

mod storage;
mod types;
mod audit_exporter;

use types::{ClaimEvent, AuthorizedAddressSet, AddressWhitelistRequested, AuthorizedPayoutAddress, AddressWhitelistRequest};
use storage::{get_claim_history, set_claim_history, get_authorized_payout_address as storage_get_authorized_payout_address, set_authorized_payout_address as storage_set_authorized_payout_address, get_pending_address_request as storage_get_pending_address_request, set_pending_address_request as storage_set_pending_address_request, remove_pending_address_request as storage_remove_pending_address_request, get_timelock_duration};

#[contract]
pub struct VestingVault;

#[contractimpl]
impl VestingVault {

    pub fn claim(e: Env, user: Address, vesting_id: u32, amount: i128) {
        user.require_auth();

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

        // Emit event for the request
        e.events().publish(
            AddressWhitelistRequested {
                beneficiary: beneficiary.clone(),
                requested_address: authorized_address.clone(),
                requested_at: current_time,
                effective_at,
            },
            (),
        );
    }

    /// Confirms and activates a pending authorized payout address request
    /// Can only be called after the 48-hour timelock period has passed
    pub fn confirm_authorized_payout_address(e: Env, beneficiary: Address) {
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
            AuthorizedAddressSet {
                beneficiary: beneficiary.clone(),
                authorized_address: pending_request.requested_address.clone(),
                effective_at: pending_request.effective_at,
            },
            (),
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
        e.storage().instance().remove(&(storage::AUTHORIZED_PAYOUT_ADDRESS, beneficiary));
        
        // Also remove any pending request
        storage_remove_pending_address_request(&e, &beneficiary);
    }

    // 🔍 helper getter (needed for exporter)
    pub fn get_all_claims(e: Env) -> Vec<ClaimEvent> {
        get_claim_history(&e)
    }
}