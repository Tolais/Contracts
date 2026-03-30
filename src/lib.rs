#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, Address, Env, String, Symbol, token};

pub mod receipt;
pub mod goal_escrow;

// =============================================
// DATA KEYS
// =============================================

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    VestingSchedule(u32),
    VestingScheduleCount,
    GroupReserve,

    // #149 #96: Gas Fee Subsidy
    GasSubsidyTracker,
    GasTreasuryBalance,
}

// =============================================
// GAS SUBSIDY TRACKER (#149 #96)
// =============================================

#[contracttype]
#[derive(Clone)]
pub struct GasSubsidyTracker {
    pub total_subsidized: u32,        // How many users have received subsidy
    pub max_subsidies: u32,           // Limit (e.g. 100 early users)
    pub min_xlm_balance: u128,        // Threshold below which we subsidize (5 XLM)
}

// =============================================
// GRANT IMPACT METADATA (from previous issue)
// =============================================

#[contracttype]
#[derive(Clone)]
pub struct GrantImpactMetadata {
    pub grant_id: u64,
    pub proposal_title: String,
    pub milestone_count: u32,
    pub impact_description: String,
    pub category: Option<String>,
    pub requested_by: Address,
    pub approved_at: u64,
}

// =============================================
// VESTING SCHEDULE
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
    pub grant_impact: Option<GrantImpactMetadata>,   // From previous issue
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
        grant_id: Option<u64>,
        proposal_title: Option<String>,
        impact_description: Option<String>,
        category: Option<String>,
    ) -> u32;

    fn claim(env: Env, beneficiary: Address, schedule_id: u32) -> u128;

    // NEW: Gas-subsidized claim for early users
    fn claim_with_subsidy(env: Env, beneficiary: Address, schedule_id: u32) -> u128;

    // Admin functions for gas treasury
    fn deposit_gas_treasury(env: Env, admin: Address, amount: u128);
    fn get_gas_subsidy_info(env: Env) -> GasSubsidyTracker;

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

        // Initialize gas subsidy tracker
        let tracker = GasSubsidyTracker {
            total_subsidized: 0,
            max_subsidies: 100,
            min_xlm_balance: 5_0000000, // 5 XLM (7 decimals)
        };
        env.storage().instance().set(&DataKey::GasSubsidyTracker, &tracker);
        env.storage().instance().set(&DataKey::GasTreasuryBalance, &0u128);
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

        let mut count: u32 = env.storage().instance().get(&DataKey::VestingScheduleCount).unwrap_or(0);
        count += 1;

        let grant_impact = if let Some(id) = grant_id {
            Some(GrantImpactMetadata {
                grant_id: id,
                proposal_title: proposal_title.unwrap_or_else(|| String::from_str(&env, "Untitled Grant")),
                milestone_count: 0,
                impact_description: impact_description.unwrap_or_else(|| String::from_str(&env, "")),
                category,
                requested_by: beneficiary.clone(),
                approved_at: env.ledger().timestamp(),
            })
        } else {
            None
        };

        let schedule = VestingSchedule {
            id: count,
            beneficiary: beneficiary.clone(),
            total_amount,
            asset,
            start_time,
            cliff_time,
            vesting_duration,
            released: 0,
            grant_impact,
        };

        env.storage().instance().set(&DataKey::VestingSchedule(count), &schedule);
        env.storage().instance().set(&DataKey::VestingScheduleCount, &count);

        env.events().publish(
            (Symbol::new(&env, "vesting_schedule_created"),),
            (count, beneficiary, total_amount)
        );

        count
    }

    fn claim(env: Env, beneficiary: Address, schedule_id: u32) -> u128 {
        beneficiary.require_auth();

        let mut schedule: VestingSchedule = env.storage()
            .instance()
            .get(&DataKey::VestingSchedule(schedule_id))
            .unwrap_or_else(|| panic!("Schedule not found"));

        if schedule.beneficiary != beneficiary {
            panic!("Not the beneficiary");
        }

        let current_time = env.ledger().timestamp();
        let vested_amount = Self::calculate_vested_amount(&schedule, current_time);

        let claimable = vested_amount - schedule.released;
        if claimable == 0 {
            panic!("Nothing to claim");
        }

        // Transfer tokens
        let token_client = token::Client::new(&env, &schedule.asset);
        token_client.transfer(&env.current_contract_address(), &beneficiary, &(claimable as i128));

        schedule.released += claimable;
        env.storage().instance().set(&DataKey::VestingSchedule(schedule_id), &schedule);

        env.events().publish(
            (Symbol::new(&env, "tokens_claimed"),),
            (beneficiary, schedule_id, claimable)
        );

        claimable
    }

    // NEW: Claim with gas subsidy for early users
    fn claim_with_subsidy(env: Env, beneficiary: Address, schedule_id: u32) -> u128 {
        beneficiary.require_auth();

        let mut tracker: GasSubsidyTracker = env.storage()
            .instance()
            .get(&DataKey::GasSubsidyTracker)
            .unwrap_or(GasSubsidyTracker {
                total_subsidized: 0,
                max_subsidies: 100,
                min_xlm_balance: 5_0000000,
            });

        let claim_amount = Self::claim(env.clone(), beneficiary.clone(), schedule_id);

        // Check if we should subsidize gas
        if tracker.total_subsidized < tracker.max_subsidies {
            // In a real implementation, you would check actual XLM balance and pay exact fee.
            // This is a simplified version for demonstration.
            let mut treasury: u128 = env.storage()
                .instance()
                .get(&DataKey::GasTreasuryBalance)
                .unwrap_or(0);

            if treasury > 0 {
                let subsidy_amount = 5000000u128; // Example: 0.5 XLM subsidy

                if treasury >= subsidy_amount {
                    treasury -= subsidy_amount;
                    env.storage().instance().set(&DataKey::GasTreasuryBalance, &treasury);

                    tracker.total_subsidized += 1;
                    env.storage().instance().set(&DataKey::GasSubsidyTracker, &tracker);

                    env.events().publish(
                        (Symbol::new(&env, "gas_subsidy_used"),),
                        (beneficiary, schedule_id, subsidy_amount)
                    );
                }
            }
        }

        claim_amount
    }

    fn deposit_gas_treasury(env: Env, admin: Address, amount: u128) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("Only admin can deposit to gas treasury");
        }

        let mut treasury: u128 = env.storage()
            .instance()
            .get(&DataKey::GasTreasuryBalance)
            .unwrap_or(0);

        treasury += amount;
        env.storage().instance().set(&DataKey::GasTreasuryBalance, &treasury);

        env.events().publish(
            (Symbol::new(&env, "gas_treasury_deposited"),),
            (admin, amount)
        );
    }

    fn get_gas_subsidy_info(env: Env) -> GasSubsidyTracker {
        env.storage()
            .instance()
            .get(&DataKey::GasSubsidyTracker)
            .unwrap_or(GasSubsidyTracker {
                total_subsidized: 0,
                max_subsidies: 100,
                min_xlm_balance: 5_0000000,
            })
    }

    fn get_grant_impact(env: Env, schedule_id: u32) -> Option<GrantImpactMetadata> {
        let schedule: VestingSchedule = env.storage()
            .instance()
            .get(&DataKey::VestingSchedule(schedule_id))
            .unwrap_or_else(|| panic!("Schedule not found"));

        schedule.grant_impact
    }

    // Helper function to calculate vested amount (stub - implement your logic)
    fn calculate_vested_amount(schedule: &VestingSchedule, current_time: u64) -> u128 {
        // Your existing vesting calculation logic here
        if current_time < schedule.start_time {
            return 0;
        }
        // ... implement linear