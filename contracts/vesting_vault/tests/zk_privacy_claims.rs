#![cfg(test)]

use soroban_sdk::contractimport;
use soroban_sdk::{vec, Address, Env, BytesN};

mod contract {
    use super::*;
    contractimport!(
        file = "target/wasm32-unknown-unknown/release/vesting_vault.wasm"
    );
}

use contract::VestingVault;

#[test]
fn test_create_commitment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let vesting_id = 1u32;
    let amount = 1000i128;
    let commitment_hash = [1u8; 32];
    
    // Test creating a commitment
    client.create_commitment(&user, &vesting_id, &amount, &commitment_hash);
    
    // Verify the commitment exists
    let commitment = client.get_commitment_info(&commitment_hash);
    assert!(commitment.is_some());
    
    let retrieved_commitment = commitment.unwrap();
    assert_eq!(retrieved_commitment.vesting_id, vesting_id);
    assert_eq!(retrieved_commitment.amount, amount);
    assert!(!retrieved_commitment.is_used);
    
    // Test duplicate commitment creation should fail
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"create_commitment",
        vec![&env, user.clone(), vesting_id, amount, commitment_hash],
    );
    assert!(result.is_err());
}

#[test]
fn test_nullifier_prevention() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let nullifier = contract::Nullifier { hash: [2u8; 32] };
    
    // Initially nullifier should not be used
    assert!(!client.is_nullifier_used_public(&nullifier));
    
    // Mark nullifier as used (simulating a private claim)
    // Note: In actual implementation, this would be done through private_claim
    // For testing, we'll need to use the storage functions directly
    
    // Test that the same nullifier cannot be used again
    // This would be tested through the private_claim function
}

#[test]
fn test_merkle_root_management() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let merkle_root = [3u8; 32];
    
    // Add a Merkle root
    client.add_merkle_root_admin(&admin, &merkle_root);
    
    // Verify the Merkle root exists
    let roots = client.get_merkle_roots();
    assert!(roots.contains(&merkle_root));
    
    // Test duplicate Merkle root should fail
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"add_merkle_root_admin",
        vec![&env, admin.clone(), merkle_root],
    );
    assert!(result.is_err());
}

#[test]
fn test_private_claim_flow() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let admin = Address::generate(&env);
    let vesting_id = 1u32;
    let amount = 1000i128;
    let commitment_hash = [4u8; 32];
    let merkle_root = [5u8; 32];
    let nullifier = contract::Nullifier { hash: [6u8; 32] };
    
    // Setup: Create commitment
    client.create_commitment(&user, &vesting_id, &amount, &commitment_hash);
    
    // Setup: Add Merkle root
    client.add_merkle_root_admin(&admin, &merkle_root);
    
    // Create ZK proof (placeholder for actual proof)
    let zk_proof = contract::ZKClaimProof {
        commitment_hash,
        nullifier_hash: nullifier.hash,
        merkle_root,
        proof_data: vec![&env],
    };
    
    // Execute private claim
    client.private_claim(&zk_proof, &nullifier, &amount);
    
    // Verify nullifier is now used
    assert!(client.is_nullifier_used_public(&nullifier));
    
    // Verify commitment is marked as used
    let commitment = client.get_commitment_info(&commitment_hash);
    assert!(commitment.is_some());
    assert!(commitment.unwrap().is_used);
    
    // Verify privacy claim history
    let privacy_history = client.get_privacy_claim_history();
    assert!(!privacy_history.is_empty());
    
    let last_claim = privacy_history.last().unwrap();
    assert_eq!(last_claim.amount, amount);
    assert_eq!(last_claim.vesting_id, vesting_id);
    assert!(last_claim.is_private);
}

#[test]
fn test_private_claim_double_spending_prevention() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let admin = Address::generate(&env);
    let vesting_id = 1u32;
    let amount = 1000i128;
    let commitment_hash = [7u8; 32];
    let merkle_root = [8u8; 32];
    let nullifier = contract::Nullifier { hash: [9u8; 32] };
    
    // Setup: Create commitment and add Merkle root
    client.create_commitment(&user, &vesting_id, &amount, &commitment_hash);
    client.add_merkle_root_admin(&admin, &merkle_root);
    
    // Create ZK proof
    let zk_proof = contract::ZKClaimProof {
        commitment_hash,
        nullifier_hash: nullifier.hash,
        merkle_root,
        proof_data: vec![&env],
    };
    
    // Execute first private claim
    client.private_claim(&zk_proof, &nullifier, &amount);
    
    // Attempt second claim with same nullifier should fail
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"private_claim",
        vec![&env, zk_proof, nullifier, amount],
    );
    assert!(result.is_err());
}

#[test]
fn test_private_claim_invalid_commitment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let amount = 1000i128;
    let non_existent_commitment_hash = [10u8; 32];
    let merkle_root = [11u8; 32];
    let nullifier = contract::Nullifier { hash: [12u8; 32] };
    
    // Setup: Add Merkle root
    client.add_merkle_root_admin(&admin, &merkle_root);
    
    // Create ZK proof with non-existent commitment
    let zk_proof = contract::ZKClaimProof {
        commitment_hash: non_existent_commitment_hash,
        nullifier_hash: nullifier.hash,
        merkle_root,
        proof_data: vec![&env],
    };
    
    // Private claim should fail
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"private_claim",
        vec![&env, zk_proof, nullifier, amount],
    );
    assert!(result.is_err());
}

#[test]
fn test_private_claim_invalid_merkle_root() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let vesting_id = 1u32;
    let amount = 1000i128;
    let commitment_hash = [13u8; 32];
    let invalid_merkle_root = [14u8; 32];
    let nullifier = contract::Nullifier { hash: [15u8; 32] };
    
    // Setup: Create commitment
    client.create_commitment(&user, &vesting_id, &amount, &commitment_hash);
    
    // Create ZK proof with invalid Merkle root
    let zk_proof = contract::ZKClaimProof {
        commitment_hash,
        nullifier_hash: nullifier.hash,
        merkle_root: invalid_merkle_root,
        proof_data: vec![&env],
    };
    
    // Private claim should fail
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"private_claim",
        vec![&env, zk_proof, nullifier, amount],
    );
    assert!(result.is_err());
}

#[test]
fn test_private_claim_amount_mismatch() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let admin = Address::generate(&env);
    let vesting_id = 1u32;
    let commitment_amount = 1000i128;
    let claim_amount = 2000i128; // Different amount
    let commitment_hash = [16u8; 32];
    let merkle_root = [17u8; 32];
    let nullifier = contract::Nullifier { hash: [18u8; 32] };
    
    // Setup: Create commitment with 1000 amount
    client.create_commitment(&user, &vesting_id, &commitment_amount, &commitment_hash);
    client.add_merkle_root_admin(&admin, &merkle_root);
    
    // Create ZK proof
    let zk_proof = contract::ZKClaimProof {
        commitment_hash,
        nullifier_hash: nullifier.hash,
        merkle_root,
        proof_data: vec![&env],
    };
    
    // Private claim with different amount should fail
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"private_claim",
        vec![&env, zk_proof, nullifier, claim_amount],
    );
    assert!(result.is_err());
}

#[test]
fn test_privacy_mode_functions() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let vesting_id = 1u32;
    
    // Test enabling privacy mode (placeholder)
    client.enable_privacy_mode(&user, &vesting_id);
    
    // Test disabling privacy mode (placeholder)
    client.disable_privacy_mode(&user, &vesting_id);
    
    // These functions are architectural placeholders for future implementation
    // They should not fail in the current implementation
}

#[test]
fn test_emergency_pause_with_private_claims() {
    let env = Env::default();
    let contract_id = env.register_contract(None, VestingVault);
    let client = VestingVaultClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    let admin = Address::generate(&env);
    let auditor1 = Address::generate(&env);
    let auditor2 = Address::generate(&env);
    let vesting_id = 1u32;
    let amount = 1000i128;
    let commitment_hash = [19u8; 32];
    let merkle_root = [20u8; 32];
    let nullifier = contract::Nullifier { hash: [21u8; 32] };
    
    // Setup: Create commitment and add Merkle root
    client.create_commitment(&user, &vesting_id, &amount, &commitment_hash);
    client.add_merkle_root_admin(&admin, &merkle_root);
    
    // Initialize auditors and trigger emergency pause
    client.initialize_auditors(&admin, &vec![&env, auditor1.clone(), auditor2.clone(), Address::generate(&env)]);
    client.request_emergency_pause(&auditor1, &"Test pause".into_val(&env));
    client.request_emergency_pause(&auditor2, &"Test pause".into_val(&env));
    
    // Create ZK proof
    let zk_proof = contract::ZKClaimProof {
        commitment_hash,
        nullifier_hash: nullifier.hash,
        merkle_root,
        proof_data: vec![&env],
    };
    
    // Private claim should fail during emergency pause
    let result = env.try_invoke_contract::<_, ()>(
        contract_id.clone(),
        &"private_claim",
        vec![&env, zk_proof, nullifier, amount],
    );
    assert!(result.is_err());
}
