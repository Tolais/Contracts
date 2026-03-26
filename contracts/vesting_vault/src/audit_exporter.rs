use soroban_sdk::{Env, Address, Vec};

use crate::types::ClaimEvent;
use crate::storage::get_claim_history;

pub fn export_all_claims(e: &Env) -> Vec<ClaimEvent> {
    get_claim_history(e)
}

pub fn export_claims_by_user(e: &Env, user: Address) -> Vec<ClaimEvent> {
    let history = get_claim_history(e);
    let mut filtered = Vec::new(e);

    for event in history.iter() {
        if event.beneficiary == user {
            filtered.push_back(event);
        }
    }

    filtered
}