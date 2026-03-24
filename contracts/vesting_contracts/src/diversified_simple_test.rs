#[cfg(test)]
mod diversified_core_tests {
    use super::diversified_core::*;
    use soroban_sdk::{testutils::Address as _, vec, Address, Env};

    #[test]
    fn test_asset_basket_validation() {
        let env = Env::default();
        
        let token1 = Address::generate(&env);
        let token2 = Address::generate(&env);
        
        // Valid basket (50% + 50% = 100%)
        let mut valid_basket = vec![&env];
        valid_basket.push_back(AssetAllocation {
            asset_id: token1.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 5000, // 50%
        });
        valid_basket.push_back(AssetAllocation {
            asset_id: token2.clone(),
            total_amount: 500,
            released_amount: 0,
            locked_amount: 0,
            percentage: 5000, // 50%
        });
        
        assert!(validate_asset_basket(&valid_basket));
        
        // Invalid basket (50% + 30% = 80% ≠ 100%)
        let mut invalid_basket = vec![&env];
        invalid_basket.push_back(AssetAllocation {
            asset_id: token1.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 5000, // 50%
        });
        invalid_basket.push_back(AssetAllocation {
            asset_id: token2.clone(),
            total_amount: 500,
            released_amount: 0,
            locked_amount: 0,
            percentage: 3000, // 30%
        });
        
        assert!(!validate_asset_basket(&invalid_basket));
    }

    #[test]
    fn test_diversified_vault_creation() {
        let env = Env::default();
        
        let owner = Address::generate(&env);
        let token1 = Address::generate(&env);
        let token2 = Address::generate(&env);
        
        let mut asset_basket = vec![&env];
        asset_basket.push_back(AssetAllocation {
            asset_id: token1.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 7000, // 70%
        });
        asset_basket.push_back(AssetAllocation {
            asset_id: token2.clone(),
            total_amount: 300,
            released_amount: 0,
            locked_amount: 0,
            percentage: 3000, // 30%
        });
        
        let start_time = env.ledger().timestamp();
        let end_time = start_time + 365 * 24 * 60 * 60; // 1 year
        
        let vault = create_diversified_vault(
            &env,
            owner.clone(),
            asset_basket,
            start_time,
            end_time,
        );
        
        assert_eq!(vault.owner, owner);
        assert_eq!(vault.allocations.len(), 2);
        assert_eq!(vault.start_time, start_time);
        assert_eq!(vault.end_time, end_time);
        assert!(vault.is_initialized);
    }

    #[test]
    fn test_linear_vesting_calculation() {
        let env = Env::default();
        
        let owner = Address::generate(&env);
        let token1 = Address::generate(&env);
        
        let mut asset_basket = vec![&env];
        asset_basket.push_back(AssetAllocation {
            asset_id: token1.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 10000, // 100%
        });
        
        let start_time = 1000;
        let end_time = 2000; // 1000 second duration
        
        env.ledger().with_mut(|li| {
            li.timestamp = start_time;
        });
        
        let vault = create_diversified_vault(
            &env,
            owner.clone(),
            asset_basket,
            start_time,
            end_time,
        );
        
        // At start time, should be 0
        let claimable = calculate_claimable_for_asset(&env, &vault, 0);
        assert_eq!(claimable, 0);
        
        // At 50% through vesting period
        env.ledger().with_mut(|li| {
            li.timestamp = start_time + 500; // 50% of 1000 seconds
        });
        
        let claimable = calculate_claimable_for_asset(&env, &vault, 0);
        assert_eq!(claimable, 500); // 50% of 1000
        
        // At end time, should be full amount
        env.ledger().with_mut(|li| {
            li.timestamp = end_time;
        });
        
        let claimable = calculate_claimable_for_asset(&env, &vault, 0);
        assert_eq!(claimable, 1000); // 100% of 1000
        
        // After end time, should still be full amount
        env.ledger().with_mut(|li| {
            li.timestamp = end_time + 1000;
        });
        
        let claimable = calculate_claimable_for_asset(&env, &vault, 0);
        assert_eq!(claimable, 1000); // 100% of 1000
    }

    #[test]
    fn test_diversified_claiming() {
        let env = Env::default();
        
        let owner = Address::generate(&env);
        let token1 = Address::generate(&env);
        let token2 = Address::generate(&env);
        
        let mut asset_basket = vec![&env];
        asset_basket.push_back(AssetAllocation {
            asset_id: token1.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 6000, // 60%
        });
        asset_basket.push_back(AssetAllocation {
            asset_id: token2.clone(),
            total_amount: 400,
            released_amount: 0,
            locked_amount: 0,
            percentage: 4000, // 40%
        });
        
        let start_time = 1000;
        let end_time = 2000;
        
        let mut vault = create_diversified_vault(
            &env,
            owner.clone(),
            asset_basket,
            start_time,
            end_time,
        );
        
        // At 50% through vesting period
        env.ledger().with_mut(|li| {
            li.timestamp = start_time + 500;
        });
        
        let claimed = claim_diversified_tokens(&env, &mut vault);
        
        // Should have claimed 50% of each asset
        assert_eq!(claimed.len(), 2);
        
        // Check that the vault state was updated
        let allocation1 = vault.allocations.get(0).unwrap();
        let allocation2 = vault.allocations.get(1).unwrap();
        
        assert_eq!(allocation1.released_amount, 500); // 50% of 1000
        assert_eq!(allocation2.released_amount, 200); // 50% of 400
        
        // Claim again - should get nothing since no time has passed
        let claimed_again = claim_diversified_tokens(&env, &mut vault);
        assert_eq!(claimed_again.len(), 0);
        
        // Fast forward to end
        env.ledger().with_mut(|li| {
            li.timestamp = end_time;
        });
        
        let final_claim = claim_diversified_tokens(&env, &mut vault);
        assert_eq!(final_claim.len(), 2);
        
        // Check final state
        let allocation1 = vault.allocations.get(0).unwrap();
        let allocation2 = vault.allocations.get(1).unwrap();
        
        assert_eq!(allocation1.released_amount, 1000); // 100% of 1000
        assert_eq!(allocation2.released_amount, 400);  // 100% of 400
    }
}