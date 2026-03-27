use soroban_sdk::{Env, Vec, Address};
use crate::types::{ClaimEvent, AuthorizedPayoutAddress, AddressWhitelistRequest};

pub const CLAIM_HISTORY: &str = "CLAIM_HISTORY";
pub const AUTHORIZED_PAYOUT_ADDRESS: &str = "AUTHORIZED_PAYOUT_ADDRESS";
pub const PENDING_ADDRESS_REQUEST: &str = "PENDING_ADDRESS_REQUEST";

// 48 hours in seconds
const TIMELOCK_DURATION: u64 = 172_800;

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