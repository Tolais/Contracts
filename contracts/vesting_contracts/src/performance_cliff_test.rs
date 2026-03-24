#[cfg(test)]
mod performance_cliff_tests {
    use super::*;
    use soroban_sdk::{
        testutils::Address as TestAddress,
        testutils::Ledger as TestLedger,
        Address,
        Env,
        Symbol,
    };

    #[test]
    fn test_performance_cliff_creation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        env.mock_auths(&[(&admin, &Symbol::new(&env, "admin"))]);

        VestingContract::initialize(env.clone(), admin.clone(), 1000000);

        // Create a performance cliff with TVL condition
        let oracle_address = Address::generate(&env);
        let tvl_condition = OracleClient::create_tvl_condition(
            oracle_address.clone(),
            1000000, // $1M TVL target
            ComparisonOperator::GreaterThanOrEqual
        );

        let conditions = vec![&env, tvl_condition];
        let cliff = PerformanceCliff {
            conditions: conditions.clone(),
            require_all: true, // All conditions must be met
            fallback_time: 1640995200, // Jan 1, 2022 fallback
        };

        // Create vault with performance cliff
        let vault_id = VestingContract::create_vault_with_cliff(
            env.clone(),
            beneficiary.clone(),
            100000,
            1640995200, // start time
            1672531200, // end time (1 year later)
            1000, // keeper fee
            true, // revocable
            false, // not transferable
            0, // step duration (linear)
            cliff.clone()
        );

        // Verify cliff was set
        let retrieved_cliff = VestingContract::get_performance_cliff(env.clone(), vault_id);
        assert!(retrieved_cliff.is_some());

        // Check cliff status (should be false since oracle returns 0)
        let cliff_passed = VestingContract::is_cliff_passed(env.clone(), vault_id);
        assert!(!cliff_passed);

        // Verify no tokens are claimable before cliff is passed
        let claimable = VestingContract::get_claimable_amount(env.clone(), vault_id);
        assert_eq!(claimable, 0);
    }

    #[test]
    fn test_multiple_oracle_conditions() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        env.mock_auths(&[(&admin, &Symbol::new(&env, "admin"))]);

        VestingContract::initialize(env.clone(), admin.clone(), 1000000);

        // Create multiple conditions
        let tvl_oracle = Address::generate(&env);
        let price_oracle = Address::generate(&env);

        let tvl_condition = OracleClient::create_tvl_condition(
            tvl_oracle,
            1000000,
            ComparisonOperator::GreaterThanOrEqual
        );

        let price_condition = OracleClient::create_price_condition(
            price_oracle,
            100, // $100 price target
            ComparisonOperator::GreaterThan,
            Some(Symbol::new(&env, "TOKEN"))
        );

        let conditions = vec![&env, tvl_condition, price_condition];

        // Test AND logic (require all)
        let and_cliff = PerformanceCliff {
            conditions: conditions.clone(),
            require_all: true,
            fallback_time: 1640995200,
        };

        // Test OR logic (require any)
        let or_cliff = PerformanceCliff {
            conditions: conditions.clone(),
            require_all: false,
            fallback_time: 1640995200,
        };

        let vault_id_and = VestingContract::create_vault_with_cliff(
            env.clone(),
            beneficiary.clone(),
            100000,
            1640995200,
            1672531200,
            1000,
            true,
            false,
            0,
            and_cliff
        );

        let vault_id_or = VestingContract::create_vault_with_cliff(
            env.clone(),
            beneficiary.clone(),
            100000,
            1640995200,
            1672531200,
            1000,
            true,
            false,
            0,
            or_cliff
        );

        // Both should fail since oracle returns 0
        assert!(!VestingContract::is_cliff_passed(env.clone(), vault_id_and));
        assert!(!VestingContract::is_cliff_passed(env.clone(), vault_id_or));
    }

    #[test]
    fn test_fallback_time_behavior() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        env.mock_auths(&[(&admin, &Symbol::new(&env, "admin"))]);

        VestingContract::initialize(env.clone(), admin.clone(), 1000000);

        // Create cliff with past fallback time
        let oracle_address = Address::generate(&env);
        let condition = OracleClient::create_tvl_condition(
            oracle_address,
            1000000,
            ComparisonOperator::GreaterThanOrEqual
        );

        let conditions = vec![&env, condition];
        let cliff = PerformanceCliff {
            conditions: conditions.clone(),
            require_all: true,
            fallback_time: 1000000, // Past timestamp
        };

        let vault_id = VestingContract::create_vault_with_cliff(
            env.clone(),
            beneficiary.clone(),
            100000,
            1640995200,
            1672531200,
            1000,
            true,
            false,
            0,
            cliff
        );

        // Cliff should pass due to fallback time
        let cliff_passed = VestingContract::is_cliff_passed(env.clone(), vault_id);
        assert!(cliff_passed);

        // Tokens should be claimable (linear vesting from start_time)
        env.ledger().set_timestamp(1640995200 + 86400); // 1 day after start
        let claimable = VestingContract::get_claimable_amount(env.clone(), vault_id);
        assert!(claimable > 0);
    }

    #[test]
    fn test_milestone_with_performance_cliff() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        env.mock_auths(&[(&admin, &Symbol::new(&env, "admin"))]);

        VestingContract::initialize(env.clone(), admin.clone(), 1000000);

        // Create performance cliff
        let oracle_address = Address::generate(&env);
        let condition = OracleClient::create_tvl_condition(
            oracle_address,
            1000000,
            ComparisonOperator::GreaterThanOrEqual
        );

        let conditions = vec![&env, condition];
        let cliff = PerformanceCliff {
            conditions: conditions.clone(),
            require_all: true,
            fallback_time: 1640995200,
        };

        let vault_id = VestingContract::create_vault_with_cliff(
            env.clone(),
            beneficiary.clone(),
            100000,
            1640995200,
            1672531200,
            1000,
            true,
            false,
            0,
            cliff
        );

        // Set milestones
        let milestone1 = Milestone {
            id: 1,
            percentage: 25,
            is_unlocked: false,
        };
        let milestone2 = Milestone {
            id: 2,
            percentage: 50,
            is_unlocked: false,
        };

        let milestones = vec![&env, milestone1, milestone2];
        VestingContract::set_milestones(env.clone(), vault_id, milestones);

        // Even with milestones, no tokens should be claimable before cliff
        let claimable = VestingContract::get_claimable_amount(env.clone(), vault_id);
        assert_eq!(claimable, 0);

        // Unlock first milestone after cliff passes
        VestingContract::unlock_milestone(env.clone(), vault_id, 1);

        // Still no tokens claimable since cliff not passed
        let claimable = VestingContract::get_claimable_amount(env.clone(), vault_id);
        assert_eq!(claimable, 0);
    }
}
