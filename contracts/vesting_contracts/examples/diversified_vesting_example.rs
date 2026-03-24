/// Example demonstrating diversified vesting functionality
/// 
/// This example shows how the new diversified vesting system works:
/// 1. Create a basket of multiple assets (e.g., 50% ProjectToken, 25% XLM, 25% USDC)
/// 2. Vest all assets simultaneously according to the same schedule
/// 3. Claim all assets proportionally as they vest
/// 
/// Benefits:
/// - Reduces exposure to single token volatility
/// - More attractive compensation packages for senior developers
/// - Stable financial planning with diversified assets

use soroban_sdk::{contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone)]
pub struct AssetAllocation {
    pub asset_id: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub locked_amount: i128,
    pub percentage: u32, // Percentage in basis points (10000 = 100%)
}

#[contracttype]
#[derive(Clone)]
pub struct DiversifiedVault {
    pub allocations: Vec<AssetAllocation>,
    pub owner: Address,
    pub start_time: u64,
    pub end_time: u64,
    pub creation_time: u64,
    pub is_initialized: bool,
}

/// Example: Create a diversified vesting schedule
/// 50% Project Token, 25% XLM, 25% USDC
pub fn create_example_diversified_vault(env: &Env) -> DiversifiedVault {
    let owner = Address::generate(env);
    let project_token = Address::generate(env);
    let xlm_token = Address::generate(env);
    let usdc_token = Address::generate(env);
    
    // Create asset basket
    let mut asset_basket = Vec::new(env);
    
    // 50% Project Token (10,000 tokens)
    asset_basket.push_back(AssetAllocation {
        asset_id: project_token,
        total_amount: 10_000_0000000, // 10,000 tokens with 7 decimals
        released_amount: 0,
        locked_amount: 0,
        percentage: 5000, // 50% in basis points
    });
    
    // 25% XLM (5,000 XLM)
    asset_basket.push_back(AssetAllocation {
        asset_id: xlm_token,
        total_amount: 5_000_0000000, // 5,000 XLM with 7 decimals
        released_amount: 0,
        locked_amount: 0,
        percentage: 2500, // 25% in basis points
    });
    
    // 25% USDC (5,000 USDC)
    asset_basket.push_back(AssetAllocation {
        asset_id: usdc_token,
        total_amount: 5_000_0000000, // 5,000 USDC with 7 decimals
        released_amount: 0,
        locked_amount: 0,
        percentage: 2500, // 25% in basis points
    });
    
    let start_time = env.ledger().timestamp();
    let end_time = start_time + (4 * 365 * 24 * 60 * 60); // 4 years
    
    DiversifiedVault {
        allocations: asset_basket,
        owner,
        start_time,
        end_time,
        creation_time: start_time,
        is_initialized: true,
    }
}

/// Calculate how much of each asset is claimable at current time
pub fn calculate_claimable_amounts(env: &Env, vault: &DiversifiedVault) -> Vec<(Address, i128)> {
    let mut claimable = Vec::new(env);
    let now = env.ledger().timestamp();
    
    if now <= vault.start_time {
        return claimable; // Nothing claimable yet
    }
    
    let total_duration = vault.end_time - vault.start_time;
    let elapsed = if now >= vault.end_time {
        total_duration
    } else {
        now - vault.start_time
    };
    
    for allocation in vault.allocations.iter() {
        // Linear vesting calculation
        let vested_amount = (allocation.total_amount * elapsed as i128) / total_duration as i128;
        let claimable_amount = vested_amount - allocation.released_amount;
        
        if claimable_amount > 0 {
            claimable.push_back((allocation.asset_id.clone(), claimable_amount));
        }
    }
    
    claimable
}

/// Example usage scenario
pub fn example_usage_scenario(env: &Env) {
    // Create a diversified vault
    let mut vault = create_example_diversified_vault(env);
    
    println!("Created diversified vault with {} assets", vault.allocations.len());
    
    // Simulate time passing - 1 year later
    env.ledger().with_mut(|li| {
        li.timestamp = vault.start_time + (365 * 24 * 60 * 60); // 1 year later
    });
    
    // Check claimable amounts after 1 year (25% of 4-year vesting)
    let claimable = calculate_claimable_amounts(env, &vault);
    
    println!("After 1 year (25% vested):");
    for (asset_id, amount) in claimable.iter() {
        println!("  Asset {}: {} tokens claimable", asset_id, amount);
    }
    
    // Expected output:
    // - Project Token: 2,500 tokens (25% of 10,000)
    // - XLM: 1,250 tokens (25% of 5,000)  
    // - USDC: 1,250 tokens (25% of 5,000)
    
    // Simulate claiming (would update vault.allocations[i].released_amount)
    // In real implementation, this would also transfer tokens to beneficiary
    
    // Simulate time passing - 4 years later (fully vested)
    env.ledger().with_mut(|li| {
        li.timestamp = vault.end_time;
    });
    
    let final_claimable = calculate_claimable_amounts(env, &vault);
    
    println!("After 4 years (100% vested):");
    for (asset_id, amount) in final_claimable.iter() {
        println!("  Asset {}: {} tokens claimable", asset_id, amount);
    }
    
    // Expected output:
    // - Project Token: 10,000 tokens (100% of 10,000)
    // - XLM: 5,000 tokens (100% of 5,000)
    // - USDC: 5,000 tokens (100% of 5,000)
}

/// Key benefits of diversified vesting:
/// 
/// 1. **Risk Reduction**: Instead of being exposed to a single token's volatility,
///    beneficiaries receive a diversified basket of assets.
/// 
/// 2. **Stable Value**: Even if the project token drops significantly, the XLM and
///    USDC portions provide stability and maintain value.
/// 
/// 3. **Attractive Compensation**: Senior developers can plan their finances better
///    knowing they'll receive stable assets alongside project tokens.
/// 
/// 4. **Flexible Composition**: Asset baskets can be customized per beneficiary
///    (e.g., executives might get 70% project token, 30% stablecoins).
/// 
/// 5. **Simultaneous Vesting**: All assets vest according to the same schedule,
///    maintaining the intended allocation percentages over time.
/// 
/// Example scenarios:
/// - Junior Developer: 30% ProjectToken, 35% XLM, 35% USDC (more stable)
/// - Senior Developer: 50% ProjectToken, 25% XLM, 25% USDC (balanced)
/// - Executive: 70% ProjectToken, 15% XLM, 15% USDC (more upside exposure)
/// - Advisor: 40% ProjectToken, 30% XLM, 30% USDC (conservative)