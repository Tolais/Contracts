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

/// Validates that asset basket percentages sum to 10000 (100%)
pub fn validate_asset_basket(basket: &Vec<AssetAllocation>) -> bool {
    let total_percentage: u32 = basket.iter().map(|a| a.percentage).sum();
    total_percentage == 10000
}

/// Calculates claimable amount for a specific asset based on linear vesting
pub fn calculate_claimable_for_asset(
    env: &Env,
    vault: &DiversifiedVault,
    asset_index: usize,
) -> i128 {
    let allocation = vault.allocations.get(asset_index.try_into().unwrap()).unwrap();
    let now = env.ledger().timestamp();
    
    if now <= vault.start_time {
        return 0;
    }
    if now >= vault.end_time {
        return allocation.total_amount;
    }
    
    let duration = (vault.end_time - vault.start_time) as i128;
    let elapsed = (now - vault.start_time) as i128;
    
    (allocation.total_amount * elapsed) / duration
}

/// Creates a diversified vault with multiple assets
pub fn create_diversified_vault(
    env: &Env,
    owner: Address,
    asset_basket: Vec<AssetAllocation>,
    start_time: u64,
    end_time: u64,
) -> DiversifiedVault {
    if !validate_asset_basket(&asset_basket) {
        panic!("Asset basket percentages must sum to 10000 (100%)");
    }
    
    if asset_basket.is_empty() {
        panic!("Asset basket cannot be empty");
    }
    
    if start_time >= end_time {
        panic!("Start time must be before end time");
    }
    
    DiversifiedVault {
        allocations: asset_basket,
        owner,
        start_time,
        end_time,
        creation_time: env.ledger().timestamp(),
        is_initialized: true,
    }
}

/// Claims all available tokens from a diversified vault
pub fn claim_diversified_tokens(
    env: &Env,
    vault: &mut DiversifiedVault,
) -> Vec<(Address, i128)> {
    let mut claimed_assets = Vec::new(env);
    
    for (i, allocation) in vault.allocations.iter().enumerate() {
        let vested_amount = calculate_claimable_for_asset(env, vault, i);
        let claimable_amount = vested_amount - allocation.released_amount;
        
        if claimable_amount > 0 {
            let mut updated_allocation = allocation.clone();
            updated_allocation.released_amount += claimable_amount;
            vault.allocations.set(i.try_into().unwrap(), updated_allocation);
            
            claimed_assets.push_back((allocation.asset_id.clone(), claimable_amount));
        }
    }
    
    claimed_assets
}