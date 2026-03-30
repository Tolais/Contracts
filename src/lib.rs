#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, Address, Env, String, Symbol, token};

pub mod receipt;      // from previous issue #233
pub mod goal_escrow;  // from previous issue #234

// =============================================
// DATA KEYS
// =============================================

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    VestingSchedule(u32),
    VestingScheduleCount,
    // ... other existing keys
}

// =============================================
// GRANT IMPACT METADATA (New for #150 #97)
// =============================================

#[contracttype]
#[derive(Clone)]
pub struct GrantImpactMetadata {
    pub grant_id: u64,                    // Reference to Grant-Stream proposal
    pub proposal_title: String,           // Human-readable title of the grant
    pub milestone_count: u32,             // Number of promised milestones
    pub impact_description: String,       // Description of expected impact
    pub category: Option<String>,         // e.g. "Infrastructure", "DeFi", "Education"
    pub requested_by: Address,            // Original grantee address
    pub approved_at: u64,                 // Timestamp when grant was approved
}

// =============================================
// VESTING SCHEDULE (Updated)
// =============================================

#[contracttype]
#[derive(Clone)]
pub struct VestingSchedule {
    pub id: u32,
    pub beneficiary: Address,
    pub total_amount: u128,
    pub asset: Address,
    pub start_time: u64,
    pub cliff_time: u64,
    pub vesting_duration: u64,
    pub released: u128,

    // NEW: Link to Grant-Stream for full lifecycle visibility
    pub grant_impact: Option<GrantImpactMetadata>,
}

// =============================================
// CONTRACT TRAIT
// =============================================

pub trait VestingVaultTrait {
    fn init(env: Env, admin: Address);

    fn create_vesting_schedule(
        env: Env,
        beneficiary: Address,
        total_amount: u128,
        asset: Address,
        start_time: u64,
        cliff_time: u64,
        vesting_duration: u64,
        grant_id: Option<u64>,              // New optional parameter
        proposal_title: Option<String>,     // New
        impact_description: Option<String>, // New
        category: Option<String>,           // New
    ) -> u32;

    // ... other existing functions ...

    // New helper to view grant impact
    fn get_grant_impact(env: Env, schedule_id: u32) -> Option<GrantImpactMetadata>;
}

// =============================================
// CONTRACT IMPLEMENTATION
// =============================================

#[contract]
pub struct VestingVault;

#[contractimpl]
impl VestingVaultTrait for VestingVault {
    fn init(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::VestingScheduleCount, &0u32);
    }

    fn create_vesting_schedule(
        env: Env,
        beneficiary: Address,
        total_amount: u128,
        asset: Address,
        start_time: u64,
        cliff_time: u64,
        vesting_duration: u64,
        grant_id: Option<u64>,
        proposal_title: Option<String>,
        impact_description: Option<String>,
        category: Option<String>,
    ) -> u32 {
        beneficiary.require_auth();

        let mut schedule_count: u32 = env.storage().instance().get(&DataKey::VestingScheduleCount).unwrap_or(0);
        schedule_count += 1;

        // Build grant impact metadata if grant_id is provided
        let grant_impact = if let Some(id) = grant_id {
            Some(GrantImpactMetadata {
                grant_id: id,
                proposal_title: proposal_title.unwrap_or_else(|| String::from_str(&env, "Untitled Grant")),
                milestone_count: 0, // Can be updated later via another function if needed
                impact_description: impact_description.unwrap_or_else(|| String::from_str(&env, "")),
                category,
                requested_by: beneficiary.clone(),
                approved_at: env.ledger().timestamp(),
            })
        } else {
            None
        };

        let schedule = VestingSchedule {
            id: schedule_count,
            beneficiary: beneficiary.clone(),
            total_amount,
            asset: asset.clone(),
            start_time,
            cliff_time,
            vesting_duration,
            released: 0,
            grant_impact,                    // ← New field populated
        };

        env.storage().instance().set(&DataKey::VestingSchedule(schedule_count), &schedule);
        env.storage().instance().set(&DataKey::VestingScheduleCount, &schedule_count);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "vesting_schedule_created"),),
            (schedule_count, beneficiary, total_amount, grant_id)
        );

        schedule_count
    }

    fn get_grant_impact(env: Env, schedule_id: u32) -> Option<GrantImpactMetadata> {
        let schedule: VestingSchedule = env.storage()
            .instance()
            .get(&DataKey::VestingSchedule(schedule_id))
            .unwrap_or_else(|| panic!("Vesting schedule not found"));

        schedule.grant_impact
    }

    // ... Keep all your existing functions below unchanged ...
    // (release, claim, get_schedule, etc.)
}

// Keep your existing test module or fuzz tests at the bottom if any