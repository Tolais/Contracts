#![cfg(test)]

use soroban_sdk::{symbol_short, Address, Env, String, U256, Vec};
use crate::{
    contractimpl, VestingContract,
    certificate_registry::{
        VestingCertificateRegistry, CompletedVestCertificate, CertificateQuery, WorkVerification,
    },
    Vault, AssetAllocation, DataKey,
};

#[test]
fn test_certificate_registration() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token = Address::generate(&env);
    
    // Setup contract
    let contract_id = env.register_contract(None, VestingContract);
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Initialize contract
    client.initialize(&admin, &token);
    
    // Create a completed vault
    let now = env.ledger().timestamp();
    let start_time = now - 100000; // Started in the past
    let end_time = now - 1000;      // Ended in the past (fully vested)
    
    let allocation = AssetAllocation {
        asset_id: token,
        total_amount: 1000,
        released_amount: 1000, // Fully claimed
        locked_amount: 0,
        percentage: 10000,
    };
    let mut allocations = Vec::new(&env);
    allocations.push_back(allocation);
    
    let vault = Vault {
        allocations,
        keeper_fee: 0,
        staked_amount: 0,
        owner: beneficiary,
        delegate: None,
        title: String::from_str(&env, "Test Vault"),
        start_time,
        end_time,
        creation_time: start_time,
        step_duration: 0,
        is_initialized: true,
        is_irrevocable: false,
        is_transferable: false,
        is_frozen: false,
    };
    
    // Register certificate
    let certificate_id = VestingCertificateRegistry::register_completed_vest(
        env.clone(),
        1,
        beneficiary,
        vault.clone(),
        1000,
        1000,
        vec![&env, token],
        None,
    );
    
    // Verify certificate was created
    let certificate = VestingCertificateRegistry::get_certificate(env.clone(), certificate_id);
    assert_eq!(certificate.vault_id, 1);
    assert_eq!(certificate.beneficiary, beneficiary);
    assert_eq!(certificate.total_claimed, 1000);
    assert_eq!(certificate.total_assets, 1000);
    assert_eq!(certificate.proof_of_work_verified, false);
    assert!(certificate.loyalty_score > 0);
}

#[test]
fn test_work_verification() {
    let env = Env::default();
    env.mock_all_auths();
    
    let beneficiary = Address::generate(&env);
    let verifier = Address::generate(&env);
    let token = Address::generate(&env);
    
    // Setup verifier
    let contract_id = env.register_contract(None, VestingCertificateRegistry);
    let registry_client = VestingCertificateRegistryClient::new(&env, &contract_id);
    
    registry_client.set_verifier(&verifier);
    
    // Create a certificate first
    let vault = create_test_vault(&env, beneficiary, token);
    let certificate_id = registry_client.register_completed_vest(
        &1,
        &beneficiary,
        &vault,
        &1000,
        &1000,
        &vec![&env, token],
        &None,
    );
    
    // Verify work
    let work_type = String::from_str(&env, "development");
    let verification_data = String::from_str(&env, "ipfs://QmTest");
    
    let result = registry_client.verify_proof_of_work(
        &certificate_id,
        &work_type,
        &95u32, // High impact score
        &verification_data,
    );
    
    assert!(result);
    
    // Verify certificate is marked as verified
    let certificate = registry_client.get_certificate(&certificate_id);
    assert!(certificate.proof_of_work_verified);
    
    // Verify work verification data
    let verification = registry_client.get_work_verification(&certificate_id);
    assert!(verification.is_some());
    let verification = verification.unwrap();
    assert_eq!(verification.work_type, work_type);
    assert_eq!(verification.impact_score, 95);
}

#[test]
fn test_certificate_query_by_beneficiary() {
    let env = Env::default();
    env.mock_all_auths();
    
    let beneficiary1 = Address::generate(&env);
    let beneficiary2 = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, VestingCertificateRegistry);
    let registry_client = VestingCertificateRegistryClient::new(&env, &contract_id);
    
    // Create certificates for two beneficiaries
    let vault1 = create_test_vault(&env, beneficiary1, token);
    let vault2 = create_test_vault(&env, beneficiary2, token);
    
    let cert1_id = registry_client.register_completed_vest(
        &1,
        &beneficiary1,
        &vault1,
        &1000,
        &1000,
        &vec![&env, token],
        &None,
    );
    
    let cert2_id = registry_client.register_completed_vest(
        &2,
        &beneficiary2,
        &vault2,
        &2000,
        &2000,
        &vec![&env, token],
        &None,
    );
    
    // Query by beneficiary1
    let query = CertificateQuery {
        beneficiary: Some(beneficiary1),
        work_type: None,
        min_loyalty_score: None,
        time_range_start: None,
        time_range_end: None,
        verified_only: None,
    };
    
    let result = registry_client.query_certificates(&query, &0, &10);
    assert_eq!(result.total_found, 1);
    assert_eq!(result.certificates.len(), 1);
    assert_eq!(result.certificates.get(0).unwrap().beneficiary, beneficiary1);
}

#[test]
fn test_certificate_query_by_loyalty_score() {
    let env = Env::default();
    env.mock_all_auths();
    
    let beneficiary = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, VestingCertificateRegistry);
    let registry_client = VestingCertificateRegistryClient::new(&env, &contract_id);
    
    // Create certificates with different loyalty scores
    let vault1 = create_test_vault(&env, beneficiary, token);
    let vault2 = create_test_vault(&env, beneficiary, token);
    
    registry_client.register_completed_vest(
        &1,
        &beneficiary,
        &vault1,
        &1000,
        &1000,
        &vec![&env, token],
        &None,
    );
    
    registry_client.register_completed_vest(
        &2,
        &beneficiary,
        &vault2,
        &2000,
        &2000,
        &vec![&env, token],
        &None,
    );
    
    // Query with high loyalty score requirement
    let query = CertificateQuery {
        beneficiary: None,
        work_type: None,
        min_loyalty_score: Some(900), // High threshold
        time_range_start: None,
        time_range_end: None,
        verified_only: None,
    };
    
    let result = registry_client.query_certificates(&query, &0, &10);
    // Should only return certificates with loyalty score >= 900
    assert!(result.certificates.iter().all(|cert| cert.loyalty_score >= 900));
}

#[test]
fn test_certificate_query_verified_only() {
    let env = Env::default();
    env.mock_all_auths();
    
    let beneficiary = Address::generate(&env);
    let verifier = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, VestingCertificateRegistry);
    let registry_client = VestingCertificateRegistryClient::new(&env, &contract_id);
    
    registry_client.set_verifier(&verifier);
    
    // Create two certificates
    let vault1 = create_test_vault(&env, beneficiary, token);
    let vault2 = create_test_vault(&env, beneficiary, token);
    
    let cert1_id = registry_client.register_completed_vest(
        &1,
        &beneficiary,
        &vault1,
        &1000,
        &1000,
        &vec![&env, token],
        &None,
    );
    
    let cert2_id = registry_client.register_completed_vest(
        &2,
        &beneficiary,
        &vault2,
        &2000,
        &2000,
        &vec![&env, token],
        &None,
    );
    
    // Verify only one certificate
    registry_client.verify_proof_of_work(
        &cert1_id,
        &String::from_str(&env, "development"),
        &95u32,
        &String::from_str(&env, "ipfs://QmTest1"),
    );
    
    // Query for verified certificates only
    let query = CertificateQuery {
        beneficiary: None,
        work_type: None,
        min_loyalty_score: None,
        time_range_start: None,
        time_range_end: None,
        verified_only: Some(true),
    };
    
    let result = registry_client.query_certificates(&query, &0, &10);
    assert_eq!(result.total_found, 1);
    assert!(result.certificates.get(0).unwrap().proof_of_work_verified);
}

#[test]
fn test_get_beneficiary_certificates() {
    let env = Env::default();
    env.mock_all_auths();
    
    let beneficiary = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, VestingCertificateRegistry);
    let registry_client = VestingCertificateRegistryClient::new(&env, &contract_id);
    
    // Create multiple certificates for the same beneficiary
    let vault1 = create_test_vault(&env, beneficiary, token);
    let vault2 = create_test_vault(&env, beneficiary, token);
    let vault3 = create_test_vault(&env, beneficiary, token);
    
    let cert1_id = registry_client.register_completed_vest(
        &1,
        &beneficiary,
        &vault1,
        &1000,
        &1000,
        &vec![&env, token],
        &None,
    );
    
    let cert2_id = registry_client.register_completed_vest(
        &2,
        &beneficiary,
        &vault2,
        &2000,
        &2000,
        &vec![&env, token],
        &None,
    );
    
    let cert3_id = registry_client.register_completed_vest(
        &3,
        &beneficiary,
        &vault3,
        &3000,
        &3000,
        &vec![&env, token],
        &None,
    );
    
    // Get beneficiary certificates
    let beneficiary_certs = registry_client.get_beneficiary_certificates(&beneficiary);
    assert_eq!(beneficiary_certs.len(), 3);
    
    // Verify all certificate IDs are present
    let cert_ids: Vec<U256> = vec![&env, cert1_id, cert2_id, cert3_id];
    for cert_id in cert_ids.iter() {
        assert!(beneficiary_certs.contains(cert_id));
    }
}

#[test]
fn test_loyalty_score_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let beneficiary = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, VestingCertificateRegistry);
    let registry_client = VestingCertificateRegistryClient::new(&env, &contract_id);
    
    // Create a vault with perfect timing (completed exactly at end_time)
    let now = env.ledger().timestamp();
    let start_time = now - 100000;
    let end_time = now - 1000; // Just ended
    
    let allocation = AssetAllocation {
        asset_id: token,
        total_amount: 1000,
        released_amount: 1000,
        locked_amount: 0,
        percentage: 10000,
    };
    let mut allocations = Vec::new(&env);
    allocations.push_back(allocation);
    
    let vault = Vault {
        allocations,
        keeper_fee: 0,
        staked_amount: 0,
        owner: beneficiary,
        delegate: None,
        title: String::from_str(&env, "Perfect Timing Vault"),
        start_time,
        end_time,
        creation_time: start_time,
        step_duration: 0,
        is_initialized: true,
        is_irrevocable: false,
        is_transferable: false,
        is_frozen: false,
    };
    
    let certificate_id = registry_client.register_completed_vest(
        &1,
        &beneficiary,
        &vault,
        &1000,
        &1000,
        &vec![&env, token],
        &None,
    );
    
    let certificate = registry_client.get_certificate(&certificate_id);
    // Should have high loyalty score for perfect timing
    assert!(certificate.loyalty_score >= 900);
}

// Helper function to create a test vault
fn create_test_vault(env: &Env, beneficiary: Address, token: Address) -> Vault {
    let now = env.ledger().timestamp();
    let start_time = now - 100000;
    let end_time = now - 1000;
    
    let allocation = AssetAllocation {
        asset_id: token,
        total_amount: 1000,
        released_amount: 1000,
        locked_amount: 0,
        percentage: 10000,
    };
    let mut allocations = Vec::new(env);
    allocations.push_back(allocation);
    
    Vault {
        allocations,
        keeper_fee: 0,
        staked_amount: 0,
        owner: beneficiary,
        delegate: None,
        title: String::from_str(env, "Test Vault"),
        start_time,
        end_time,
        creation_time: start_time,
        step_duration: 0,
        is_initialized: true,
        is_irrevocable: false,
        is_transferable: false,
        is_frozen: false,
    }
}
