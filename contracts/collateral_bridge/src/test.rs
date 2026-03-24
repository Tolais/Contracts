use soroban_sdk::{symbol_short, Address, Env};
use crate::{CollateralBridge, CollateralBridgeClient, Lien, CollateralDataKey};

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    
    // Verify initialization
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_create_lien() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    let lender = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    
    // Mock vault data - in a real test, this would come from the vesting contract
    // For now, we'll test the lien creation logic
    
    let vault_id = 1u64;
    let locked_amount = 1000i128;
    let loan_amount = 800i128;
    let interest_rate = 1000u32; // 10%
    let maturity_time = env.ledger().timestamp() + 86400; // 1 day from now
    
    // This would require vault owner authorization in a real implementation
    // For testing purposes, we'll assume the authorization is handled
    
    let lien_id = client.create_lien(
        &vault_id,
        &lender,
        &locked_amount,
        &loan_amount,
        &interest_rate,
        &maturity_time,
    );
    
    // Verify lien was created
    let lien = client.get_lien(&lien_id);
    assert_eq!(lien.vault_id, vault_id);
    assert_eq!(lien.lender, lender);
    assert_eq!(lien.locked_amount, locked_amount);
    assert_eq!(lien.loan_amount, loan_amount);
    assert_eq!(lien.interest_rate, interest_rate);
    assert_eq!(lien.maturity_time, maturity_time);
    assert!(lien.is_active);
}

#[test]
fn test_get_vault_liens() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let lender = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    
    let vault_id = 1u64;
    
    // Create multiple liens for the same vault
    let lien_id1 = client.create_lien(
        &vault_id,
        &lender,
        &1000i128,
        &800i128,
        &1000u32,
        &(env.ledger().timestamp() + 86400),
    );
    
    let lien_id2 = client.create_lien(
        &vault_id,
        &lender,
        &500i128,
        &400i128,
        &1500u32,
        &(env.ledger().timestamp() + 172800),
    );
    
    // Get vault liens
    let vault_liens = client.get_vault_liens(&vault_id);
    assert_eq!(vault_liens.len(), 2);
    assert!(vault_liens.contains(&lien_id1));
    assert!(vault_liens.contains(&lien_id2));
}

#[test]
fn test_get_lender_liens() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let lender1 = Address::generate(&env);
    let lender2 = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    
    // Create liens for different lenders
    let lien_id1 = client.create_lien(
        &1u64,
        &lender1,
        &1000i128,
        &800i128,
        &1000u32,
        &(env.ledger().timestamp() + 86400),
    );
    
    let lien_id2 = client.create_lien(
        &2u64,
        &lender2,
        &500i128,
        &400i128,
        &1500u32,
        &(env.ledger().timestamp() + 172800),
    );
    
    let lien_id3 = client.create_lien(
        &3u64,
        &lender1,
        &750i128,
        &600i128,
        &1200u32,
        &(env.ledger().timestamp() + 259200),
    );
    
    // Get lender liens
    let lender1_liens = client.get_lender_liens(&lender1);
    assert_eq!(lender1_liens.len(), 2);
    assert!(lender1_liens.contains(&lien_id1));
    assert!(lender1_liens.contains(&lien_id3));
    
    let lender2_liens = client.get_lender_liens(&lender2);
    assert_eq!(lender2_liens.len(), 1);
    assert!(lender2_liens.contains(&lien_id2));
}

#[test]
fn test_release_lien() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    let lender = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    
    let vault_id = 1u64;
    let lien_id = client.create_lien(
        &vault_id,
        &lender,
        &1000i128,
        &800i128,
        &1000u32,
        &(env.ledger().timestamp() + 86400),
    );
    
    // Verify lien is active
    let lien = client.get_lien(&lien_id);
    assert!(lien.is_active);
    
    // Release lien (would require vault owner authorization)
    client.release_lien(&lien_id);
    
    // Verify lien is no longer active
    let lien = client.get_lien(&lien_id);
    assert!(!lien.is_active);
}

#[test]
fn test_toggle_pause() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    
    // Initially not paused
    assert!(!client.is_paused());
    
    // Pause contract
    client.toggle_pause();
    assert!(client.is_paused());
    
    // Unpause contract
    client.toggle_pause();
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    client.initialize(&admin, &vesting_contract); // Should panic
}

#[test]
#[should_panic(expected = "Contract paused")]
fn test_create_lien_when_paused() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let lender = Address::generate(&env);
    let vesting_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, CollateralBridge);
    let client = CollateralBridgeClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_contract);
    client.toggle_pause(); // Pause the contract
    
    client.create_lien(
        &1u64,
        &lender,
        &1000i128,
        &800i128,
        &1000u32,
        &(env.ledger().timestamp() + 86400),
    ); // Should panic
}
