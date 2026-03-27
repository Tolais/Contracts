#![cfg(test)]

use soroban_sdk::{symbol_short, Address, Env, Vec, String};
use vesting_vault::{VestingVault, VestingVaultClient, PathPaymentConfig, PathPaymentSimulation, PathPaymentClaimEvent};

#[test]
fn test_configure_path_payment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let intermediate_asset = Address::generate(&env);
    
    let mut path = Vec::new(&env);
    path.push_back(intermediate_asset);
    
    let min_destination_amount = 1000i128;
    
    // Configure path payment
    client.configure_path_payment(
        &admin,
        &usdc_asset,
        &min_destination_amount,
        &path
    );
    
    // Verify configuration
    let config = client.get_path_payment_config();
    assert!(config.is_some());
    
    let config = config.unwrap();
    assert_eq!(config.destination_asset, usdc_asset);
    assert_eq!(config.min_destination_amount, min_destination_amount);
    assert_eq!(config.path, path);
    assert!(config.enabled);
}

#[test]
fn test_disable_path_payment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment first
    client.configure_path_payment(&admin, &usdc_asset, &1000i128, &path);
    
    // Disable it
    client.disable_path_payment(&admin);
    
    // Verify it's disabled
    let config = client.get_path_payment_config();
    assert!(config.is_some());
    assert!(!config.unwrap().enabled);
}

#[test]
fn test_simulate_path_payment_claim_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    
    // Simulate claim
    let simulation = client.simulate_path_payment_claim(
        &user,
        &1u32,
        &1000i128,
        &Some(950i128)
    );
    
    assert!(simulation.can_execute);
    assert_eq!(simulation.source_amount, 1000i128);
    assert!(simulation.estimated_destination_amount >= 950i128); // Should be ~995 with 0.5% slippage
    assert_eq!(simulation.min_destination_amount, 950i128);
    assert_eq!(simulation.reason, String::from_str(&env, "Path payment claim available"));
    assert!(simulation.estimated_gas_fee > 0);
}

#[test]
fn test_simulate_path_payment_claim_insufficient_liquidity() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment with high minimum
    client.configure_path_payment(&admin, &usdc_asset, &999i128, &path);
    
    // Simulate claim with insufficient liquidity
    let simulation = client.simulate_path_payment_claim(
        &user,
        &1u32,
        &1000i128,
        &Some(999i128)
    );
    
    assert!(!simulation.can_execute);
    assert_eq!(simulation.source_amount, 1000i128);
    assert!(simulation.estimated_destination_amount < 999i128); // Should be ~995 with 0.5% slippage
    assert_eq!(simulation.min_destination_amount, 999i128);
    assert_eq!(simulation.reason, String::from_str(&env, "Insufficient liquidity for minimum destination amount"));
}

#[test]
fn test_simulate_path_payment_claim_not_configured() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    // Simulate claim without configuration
    let simulation = client.simulate_path_payment_claim(
        &user,
        &1u32,
        &1000i128,
        &Some(950i128)
    );
    
    assert!(!simulation.can_execute);
    assert_eq!(simulation.source_amount, 1000i128);
    assert_eq!(simulation.estimated_destination_amount, 0i128);
    assert_eq!(simulation.reason, String::from_str(&env, "Path payment not configured"));
}

#[test]
fn test_simulate_path_payment_claim_disabled() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure and then disable path payment
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    client.disable_path_payment(&admin);
    
    // Simulate claim
    let simulation = client.simulate_path_payment_claim(
        &user,
        &1u32,
        &1000i128,
        &Some(950i128)
    );
    
    assert!(!simulation.can_execute);
    assert_eq!(simulation.reason, String::from_str(&env, "Path payment feature is disabled"));
}

#[test]
fn test_claim_with_path_payment_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    
    // Execute claim with path payment
    client.claim_with_path_payment(
        &user,
        &1u32,
        &1000i128,
        &Some(950i128)
    );
    
    // Verify claim history contains the path payment claim
    let path_payment_history = client.get_path_payment_claim_history();
    assert_eq!(path_payment_history.len(), 1);
    
    let claim_event = path_payment_history.get(0).unwrap();
    assert_eq!(claim_event.beneficiary, user);
    assert_eq!(claim_event.source_amount, 1000i128);
    assert!(claim_event.destination_amount >= 950i128);
    assert_eq!(claim_event.destination_asset, usdc_asset);
    assert_eq!(claim_event.vesting_id, 1u32);
}

#[test]
fn test_claim_with_path_payment_not_configured() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    // Try to claim without configuration
    let result = env.try_invoke_contract(
        &contract_id,
        &symbol_short!("claim_with_path_payment"),
        (user.clone(), 1u32, 1000i128, Some(950i128))
    );
    
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("Path payment not configured"));
}

#[test]
fn test_claim_with_path_payment_disabled() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure and then disable path payment
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    client.disable_path_payment(&admin);
    
    // Try to claim while disabled
    let result = env.try_invoke_contract(
        &contract_id,
        &symbol_short!("claim_with_path_payment"),
        (user.clone(), 1u32, 1000i128, Some(950i128))
    );
    
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("Path payment feature is disabled"));
}

#[test]
fn test_claim_with_path_payment_insufficient_minimum() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    
    // Try to claim with insufficient minimum (higher than what liquidity can provide)
    let result = env.try_invoke_contract(
        &contract_id,
        &symbol_short!("claim_with_path_payment"),
        (user.clone(), 1u32, 1000i128, Some(999i128))
    );
    
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("insufficient liquidity for minimum destination amount"));
}

#[test]
fn test_path_payment_with_custom_path() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let intermediate_asset1 = Address::generate(&env);
    let intermediate_asset2 = Address::generate(&env);
    
    // Create a custom path: Token -> Asset1 -> Asset2 -> USDC
    let mut path = Vec::new(&env);
    path.push_back(intermediate_asset1);
    path.push_back(intermediate_asset2);
    
    // Configure path payment with custom path
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    
    // Verify the path is stored correctly
    let config = client.get_path_payment_config().unwrap();
    assert_eq!(config.path.len(), 2);
    assert_eq!(config.path.get(0), intermediate_asset1);
    assert_eq!(config.path.get(1), intermediate_asset2);
    
    // Simulate claim with custom path
    let simulation = client.simulate_path_payment_claim(
        &user,
        &1u32,
        &1000i128,
        &Some(950i128)
    );
    
    assert!(simulation.can_execute);
    assert_eq!(simulation.path.len(), 2);
}

#[test]
fn test_path_payment_fallback_to_config_minimum() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment with specific minimum
    client.configure_path_payment(&admin, &usdc_asset, &900i128, &path);
    
    // Simulate claim without providing custom minimum (should use config minimum)
    let simulation = client.simulate_path_payment_claim(
        &user,
        &1u32,
        &1000i128,
        &None::<i128>
    );
    
    assert!(simulation.can_execute);
    assert_eq!(simulation.min_destination_amount, 900i128); // Should use config minimum
}

#[test]
fn test_path_payment_zero_minimum_amount() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let usdc_asset = Address::generate(&env);
    let path = Vec::new(&env);
    
    // Configure path payment
    client.configure_path_payment(&admin, &usdc_asset, &950i128, &path);
    
    // Try to claim with zero minimum amount
    let result = env.try_invoke_contract(
        &contract_id,
        &symbol_short!("claim_with_path_payment"),
        (user.clone(), 1u32, 1000i128, Some(0i128))
    );
    
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("Minimum destination amount must be positive"));
}
