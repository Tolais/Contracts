// Milestone-gated vesting implementation
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, Vec, Map, i128, u64, u32, token
};

#[contracttype]
pub struct MilestoneData {
    pub vault_id: u64,
    pub milestones: Vec<u32>, // Percentages
    pub current_milestone: u32,
    pub triggered_milestones: Vec<u32>,
}

#[contracttype] 
pub struct MilestoneEvent {
    pub milestone_id: u32,
    pub is_triggered: bool,
    pub trigger_time: u64,
    pub triggered_by: Address,
}

// Add milestone functions to existing VestingContract
pub trait MilestoneVesting {
    fn create_milestone_vault(env: Env, owner: Address, amount: i128, milestones: Vec<u32>) -> u64;
    fn trigger_milestone(env: Env, vault_id: u64, milestone_id: u32, admin: Address);
    fn claim_milestone_tokens(env: Env, vault_id: u64) -> i128;
    fn simulate_claim(env: Env, vault_id: u64, claim_amount: Option<i128>) -> (i128, i128, i128);
}
