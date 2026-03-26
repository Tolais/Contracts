#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Address, Vec};

mod storage;
mod types;
mod audit_exporter;

use types::ClaimEvent;
use storage::{get_claim_history, set_claim_history};

#[contract]
pub struct VestingVault;

#[contractimpl]
impl VestingVault {

    pub fn claim(e: Env, user: Address, vesting_id: u32, amount: i128) {
        user.require_auth();

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

    // 🔍 helper getter (needed for exporter)
    pub fn get_all_claims(e: Env) -> Vec<ClaimEvent> {
        get_claim_history(&e)
    }
}