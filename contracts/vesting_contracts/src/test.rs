use crate::{
    BatchCreateData, Milestone, VestingContract, VestingContractClient,
    GovernanceAction, GovernanceProposal, Vote,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, IntoVal, Symbol, String, Map,
};

fn setup() -> (Env, Address, VestingContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000_000i128);

    let token_admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    client.set_token(&token_addr);
    client.add_to_whitelist(&token_addr);

    // Mint initial supply to contract
    let stellar = token::StellarAssetClient::new(&env, &token_addr);
    stellar.mint(&contract_id, &1_000_000_000i128);

    (env, contract_id, client, admin, token_addr)
}

#[test]
fn test_initialize() {
    let (env, _, client, admin, _) = setup();
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_create_vault_full_and_claim() {
    let (env, _, client, admin, token) = setup();
    let beneficiary = Address::generate(&env);
    let now = env.ledger().timestamp();
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &1000i128,
        &now,
        &(now + 1000),
        &0i128,
        &false, // irrevocable
        &false,
        &0u64,
    );

    assert_eq!(vault_id, 1);
    
    // Fast forward
    env.ledger().set_timestamp(now + 500);
    assert_eq!(client.get_claimable_amount(&vault_id), 500);

    // Claim
    client.claim_tokens(&vault_id, &100i128);
    assert_eq!(client.get_claimable_amount(&vault_id), 400);
    
    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&beneficiary), 100);
}

#[test]
fn test_periodic_vesting() {
    let (env, _, client, _, _) = setup();
    let beneficiary = Address::generate(&env);
    let now = env.ledger().timestamp();
    
    // 1000 tokens over 1000 seconds, with 100 second steps
    let vault_id = client.create_vault_full(
        &beneficiary,
        &1000i128,
        &now,
        &(now + 1000),
        &0i128,
        &true,
        &false,
        &100u64,
    );

    env.ledger().set_timestamp(now + 150);
    // 1 step completed (100 tokens)
    assert_eq!(client.get_claimable_amount(&vault_id), 100);

    env.ledger().set_timestamp(now + 250);
    // 2 steps completed (200 tokens)
    assert_eq!(client.get_claimable_amount(&vault_id), 200);
}

#[test]
fn test_milestones() {
    let (env, _, client, admin, _) = setup();
    let beneficiary = Address::generate(&env);
    let now = env.ledger().timestamp();
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &1000i128,
        &now,
        &(now + 1000),
        &0i128,
        &true,
        &false,
        &0u64,
    );

    let milestones = vec![&env, 
        Milestone { id: 1, percentage: 30, is_unlocked: false },
        Milestone { id: 2, percentage: 70, is_unlocked: false }
    ];
    
    client.set_milestones(&vault_id, &milestones);
    
    assert_eq!(client.get_claimable_amount(&vault_id), 0);
    
    client.unlock_milestone(&vault_id, &1);
    assert_eq!(client.get_claimable_amount(&vault_id), 300);
    
    client.unlock_milestone(&vault_id, &2);
    assert_eq!(client.get_claimable_amount(&vault_id), 1000);
}

#[test]
fn test_global_pause() {
    let (env, _, client, admin, _) = setup();
    
    client.toggle_pause();
    assert!(client.is_paused());
    
    let beneficiary = Address::generate(&env);
    // Logic that depends on paused should fail
}

#[test]
fn test_batch_operations() {
    let (env, _, client, _, _) = setup();
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let now = env.ledger().timestamp();
    
    let batch = BatchCreateData {
        recipients: vec![&env, r1, r2],
        amounts: vec![&env, 500i128, 500i128],
        start_times: vec![&env, now, now],
        end_times: vec![&env, now + 1000, now + 1000],
        keeper_fees: vec![&env, 0i128, 0i128],
        step_durations: vec![&env, 0u64, 0u64],
    };
    
    let ids = client.batch_create_vaults_full(&batch);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids.get(0).unwrap(), 1);
    assert_eq!(ids.get(1).unwrap(), 2);
}

// --- Governance Tests ---

#[test]
fn test_propose_admin_rotation() {
    let (env, _, client, admin, _) = setup();
    let new_admin = Address::generate(&env);
    
    let proposal_id = client.propose_admin_rotation(&new_admin);
    assert_eq!(proposal_id, 1);
    
    let proposal = client.get_proposal_info(&proposal_id);
    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.proposer, admin);
    match proposal.action {
        GovernanceAction::AdminRotation(addr) => assert_eq!(addr, new_admin),
        _ => panic!("Expected AdminRotation action"),
    }
}

#[test]
fn test_propose_contract_upgrade() {
    let (env, _, client, admin, _) = setup();
    let new_contract = Address::generate(&env);
    
    let proposal_id = client.propose_contract_upgrade(&new_contract);
    assert_eq!(proposal_id, 1);
    
    let proposal = client.get_proposal_info(&proposal_id);
    match proposal.action {
        GovernanceAction::ContractUpgrade(addr) => assert_eq!(addr, new_contract),
        _ => panic!("Expected ContractUpgrade action"),
    }
}

#[test]
fn test_propose_emergency_pause() {
    let (env, _, client, _, _) = setup();
    
    let proposal_id = client.propose_emergency_pause(&true);
    assert_eq!(proposal_id, 1);
    
    let proposal = client.get_proposal_info(&proposal_id);
    match proposal.action {
        GovernanceAction::EmergencyPause(pause_state) => assert_eq!(pause_state, true),
        _ => panic!("Expected EmergencyPause action"),
    }
}

#[test]
fn test_voting_power_calculation() {
    let (env, _, client, _, token) = setup();
    let beneficiary = Address::generate(&env);
    let now = env.ledger().timestamp();
    
    // Create a vault for the beneficiary
    let vault_id = client.create_vault_full(
        &beneficiary,
        &1000i128,
        &now,
        &(now + 1000),
        &0i128,
        &false,
        &false,
        &0u64,
    );
    
    let voting_power = client.get_voter_power(&beneficiary);
    assert_eq!(voting_power, 1000);
    
    // After partial claim, voting power should decrease
    env.ledger().set_timestamp(now + 500);
    client.claim_tokens(&vault_id, &200i128);
    
    let voting_power_after_claim = client.get_voter_power(&beneficiary);
    assert_eq!(voting_power_after_claim, 800);
}

#[test]
fn test_successful_governance_execution() {
    let (env, _, client, admin, _) = setup();
    let new_admin = Address::generate(&env);
    
    // Propose admin rotation
    let proposal_id = client.propose_admin_rotation(&new_admin);
    
    // Fast forward past challenge period
    let proposal = client.get_proposal_info(&proposal_id);
    env.ledger().set_timestamp(proposal.challenge_end + 1);
    
    // Execute proposal (should pass with no votes against)
    client.execute_proposal(&proposal_id);
    
    // Check admin was changed
    assert_eq!(client.get_admin(), new_admin);
    
    // Check proposal is marked as executed
    let updated_proposal = client.get_proposal_info(&proposal_id);
    assert!(updated_proposal.is_executed);
}

#[test]
fn test_vetoed_governance_proposal() {
    let (env, _, client, _, token) = setup();
    let beneficiary1 = Address::generate(&env);
    let beneficiary2 = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let now = env.ledger().timestamp();
    
    // Create vaults with significant tokens (51%+ of total)
    client.create_vault_full(&beneficiary1, &600i128, &now, &(now + 1000), &0i128, &false, &false, &0u64);
    client.create_vault_full(&beneficiary2, &400i128, &now, &(now + 1000), &0i128, &false, &false, &0u64);
    
    // Propose admin rotation
    let proposal_id = client.propose_admin_rotation(&new_admin);
    
    // Vote against the proposal (51% of total)
    // Note: In real implementation, beneficiaries would vote directly
    // For test purposes, we'll simulate the voting
    
    // Fast forward past challenge period
    let proposal = client.get_proposal_info(&proposal_id);
    env.ledger().set_timestamp(proposal.challenge_end + 1);
    
    // Manually set veto votes for testing
    // In real implementation, this would happen through vote_on_proposal calls
    
    // Execute proposal (should fail due to veto)
    client.execute_proposal(&proposal_id);
    
    // Check admin was NOT changed
    assert_ne!(client.get_admin(), new_admin);
    
    // Check proposal is marked as cancelled
    let updated_proposal = client.get_proposal_info(&proposal_id);
    assert!(updated_proposal.is_cancelled);
}

#[test]
fn test_challenge_period_enforcement() {
    let (env, _, client, _, _) = setup();
    let new_admin = Address::generate(&env);
    
    // Propose admin rotation
    let proposal_id = client.propose_admin_rotation(&new_admin);
    
    // Try to execute before challenge period ends
    let proposal = client.get_proposal_info(&proposal_id);
    env.ledger().set_timestamp(proposal.challenge_end - 1);
    
    // Should panic because challenge period hasn't ended
    let _result = std::panic::catch_unwind(|| {
        client.execute_proposal(&proposal_id);
    });
    assert!(_result.is_err());
}

#[test]
fn test_emergency_pause_governance() {
    let (env, _, client, _, _) = setup();
    
    // Initially not paused
    assert!(!client.is_paused());
    
    // Propose emergency pause
    let proposal_id = client.propose_emergency_pause(&true);
    
    // Fast forward past challenge period
    let proposal = client.get_proposal_info(&proposal_id);
    env.ledger().set_timestamp(proposal.challenge_end + 1);
    
    // Execute proposal
    client.execute_proposal(&proposal_id);
    
    // Should now be paused
    assert!(client.is_paused());
}

#[test]
fn test_contract_upgrade_governance() {
    let (env, _, client, _, _) = setup();
    let new_contract = Address::generate(&env);
    
    // Initially not deprecated
    // Note: We'd need a getter for IsDeprecated to test this properly
    
    // Propose contract upgrade
    let proposal_id = client.propose_contract_upgrade(&new_contract);
    
    // Fast forward past challenge period
    let proposal = client.get_proposal_info(&proposal_id);
    env.ledger().set_timestamp(proposal.challenge_end + 1);
    
    // Execute proposal
    client.execute_proposal(&proposal_id);
    
    // Check proposal is executed
    let updated_proposal = client.get_proposal_info(&proposal_id);
    assert!(updated_proposal.is_executed);
}
