#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env, IntoVal};

mod vesting_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/vesting_contracts.wasm"
    );
}

#[test]
fn test_nft_minting_and_levels() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    // Register Vesting Contract
    let vesting_id = env.register_contract_wasm(None, vesting_contract::WASM);
    let vesting_client = vesting_contract::Client::new(&env, &vesting_id);

    // Register NFT Contract
    let nft_id = env.register_contract(None, VestingStatusNFT);
    let nft_client = VestingStatusNFTClient::new(&env, &nft_id);

    // Initialize contracts
    vesting_client.initialize(&admin, &1000_000_000_000);
    nft_client.initialize(&admin, &vesting_id);

    // Set NFT minter in Vesting Contract
    vesting_client.set_nft_minter(&nft_id);

    // Setup Token
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&vesting_id, &1000);
    vesting_client.set_token(&token_id);

    // Create Vault for user: 100 tokens, 100 seconds
    let start_time = 1000;
    let end_time = 2000;
    env.ledger().with_mut(|li| li.timestamp = 0);
    
    let vault_id = vesting_client.create_vault_full(
        &user, 
        &100, 
        &start_time, 
        &end_time, 
        &0, 
        &false, 
        &false, 
        &0
    );

    // Verify initially No Badge and Level 0
    // (Actually Level 0 is returned if no vaults, but user has a vault now)
    assert_eq!(nft_client.get_level(&user), 0);

    // Advance time to 25% (1250)
    env.ledger().with_mut(|li| li.timestamp = 1250);
    
    // Claim tokens to trigger mint
    vesting_client.claim_tokens(&vault_id, &25);
    
    // Verify Level 1
    assert_eq!(nft_client.get_level(&user), 1);
    assert_eq!(nft_client.metadata(&user), String::from_str(&env, "Vesting Badge - Level 1: Steady Hand"));

    // Advance time to 50% (1500)
    env.ledger().with_mut(|li| li.timestamp = 1500);
    vesting_client.claim_tokens(&vault_id, &25);
    assert_eq!(nft_client.get_level(&user), 2);

    // Advance time to 75% (1750)
    env.ledger().with_mut(|li| li.timestamp = 1750);
    vesting_client.claim_tokens(&vault_id, &25);
    assert_eq!(nft_client.get_level(&user), 3);

    // Advance time to 100% (2000)
    env.ledger().with_mut(|li| li.timestamp = 2000);
    vesting_client.claim_tokens(&vault_id, &25);
    assert_eq!(nft_client.get_level(&user), 4);
    assert_eq!(nft_client.metadata(&user), String::from_str(&env, "Vesting Badge - Level 4: Master of Loyalty"));
}
