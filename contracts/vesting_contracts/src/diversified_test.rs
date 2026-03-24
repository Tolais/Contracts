#[cfg(test)]
mod diversified_vesting_tests {
    use crate::{AssetAllocation, VestingContract, VestingContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, vec, Address, Env, String,
    };

    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        token::Client::new(env, &env.register_stellar_asset_contract(admin.clone()))
    }

    #[test]
    fn test_diversified_vesting_basic() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        // Create three different tokens
        let token1 = create_token_contract(&env, &admin);
        let token2 = create_token_contract(&env, &admin);
        let token3 = create_token_contract(&env, &admin);

        // Initialize tokens with supply
        token1.initialize(&admin, &7, &"Token1".into_val(&env), &"TK1".into_val(&env));
        token2.initialize(&admin, &7, &"Token2".into_val(&env), &"TK2".into_val(&env));
        token3.initialize(&admin, &7, &"Token3".into_val(&env), &"TK3".into_val(&env));

        // Mint tokens to admin
        token1.mint(&admin, &10000);
        token2.mint(&admin, &10000);
        token3.mint(&admin, &10000);

        // Create vesting contract
        let contract_id = env.register_contract(None, VestingContract);
        let client = VestingContractClient::new(&env, &contract_id);

        // Initialize vesting contract
        client.initialize(&admin, &10000);

        // Create asset basket: 50% Token1, 30% Token2, 20% Token3
        let mut asset_basket = vec![&env];
        asset_basket.push_back(AssetAllocation {
            asset_id: token1.address.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 5000, // 50%
        });
        asset_basket.push_back(AssetAllocation {
            asset_id: token2.address.clone(),
            total_amount: 600,
            released_amount: 0,
            locked_amount: 0,
            percentage: 3000, // 30%
        });
        asset_basket.push_back(AssetAllocation {
            asset_id: token3.address.clone(),
            total_amount: 400,
            released_amount: 0,
            locked_amount: 0,
            percentage: 2000, // 20%
        });

        let start_time = env.ledger().timestamp();
        let end_time = start_time + 365 * 24 * 60 * 60; // 1 year

        // Create diversified vault
        let vault_id = client.create_vault_diversified_full(
            &beneficiary,
            &asset_basket,
            &start_time,
            &end_time,
            &0, // no keeper fee
            &true, // revocable
            &true, // transferable
            &0, // no step duration (linear vesting)
            &String::from_str(&env, "Diversified Vault"),
        );

        // Verify vault was created
        let vault = client.get_vault(&vault_id);
        assert_eq!(vault.owner, beneficiary);
        assert_eq!(vault.allocations.len(), 3);
        assert_eq!(vault.is_initialized, true);

        // Check initial balances
        assert_eq!(token1.balance(&beneficiary), 0);
        assert_eq!(token2.balance(&beneficiary), 0);
        assert_eq!(token3.balance(&beneficiary), 0);

        // Fast forward to 50% of vesting period
        env.ledger().with_mut(|li| {
            li.timestamp = start_time + (end_time - start_time) / 2;
        });

        // Claim tokens
        let claimed = client.claim_tokens_diversified(&vault_id);
        
        // Should have claimed 50% of each asset
        assert_eq!(claimed.len(), 3);
        
        // Check balances after claim
        assert_eq!(token1.balance(&beneficiary), 500); // 50% of 1000
        assert_eq!(token2.balance(&beneficiary), 300); // 50% of 600
        assert_eq!(token3.balance(&beneficiary), 200); // 50% of 400

        // Fast forward to end of vesting period
        env.ledger().with_mut(|li| {
            li.timestamp = end_time;
        });

        // Claim remaining tokens
        let claimed = client.claim_tokens_diversified(&vault_id);
        
        // Check final balances
        assert_eq!(token1.balance(&beneficiary), 1000); // 100% of 1000
        assert_eq!(token2.balance(&beneficiary), 600);  // 100% of 600
        assert_eq!(token3.balance(&beneficiary), 400);  // 100% of 400
    }

    #[test]
    fn test_asset_basket_validation() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        let token1 = create_token_contract(&env, &admin);
        token1.initialize(&admin, &7, &"Token1".into_val(&env), &"TK1".into_val(&env));
        token1.mint(&admin, &10000);

        let contract_id = env.register_contract(None, VestingContract);
        let client = VestingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &10000);

        // Create invalid asset basket (percentages don't sum to 100%)
        let mut invalid_basket = vec![&env];
        invalid_basket.push_back(AssetAllocation {
            asset_id: token1.address.clone(),
            total_amount: 1000,
            released_amount: 0,
            locked_amount: 0,
            percentage: 5000, // Only 50%, should be 100%
        });

        let start_time = env.ledger().timestamp();
        let end_time = start_time + 365 * 24 * 60 * 60;

        // This should panic due to invalid percentages
        let result = std::panic::catch_unwind(|| {
            client.create_vault_diversified_full(
                &beneficiary,
                &invalid_basket,
                &start_time,
                &end_time,
                &0,
                &true,
                &true,
                &0,
                &String::from_str(&env, "Invalid Vault"),
            );
        });

        assert!(result.is_err());
    }
}