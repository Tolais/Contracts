use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use vesting_contracts::{BatchCreateData, ScheduleConfig, VestingContract, VestingContractClient, MAX_DURATION, AdminAction};

fn setup(env: &Env) -> (VestingContractClient<'static>, Address) {
    env.mock_all_auths();

    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    // initialize multisig (single admin but requires proposal flow)
    let mut admins = vec![env];
    admins.push_back(admin.clone());
    client.initialize_multisig(&admins, &1u32, &1_000_000i128);

    (client, admin)
}

#[test]
fn create_vault_full_allows_max_duration() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let beneficiary = Address::generate(&env);
    let start = env.ledger().timestamp();
    let end = start + MAX_DURATION;

    let cfg = ScheduleConfig {
        owner: beneficiary.clone(),
        amount: 1_000i128,
        start_time: start,
        end_time: end,
        keeper_fee: 0i128,
        is_revocable: true,
        is_transferable: false,
        step_duration: 0u64,
    };
    let action = AdminAction::AddBeneficiary(beneficiary.clone(), cfg);
    client.propose_admin_action(&admin, &action);
}

#[test]
#[should_panic(expected = "duration exceeds MAX_DURATION")]
fn create_vault_full_rejects_over_max_duration() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let beneficiary = Address::generate(&env);
    let start = env.ledger().timestamp();
    let end = start + MAX_DURATION + 1;

    let cfg = ScheduleConfig {
        owner: beneficiary.clone(),
        amount: 1_000i128,
        start_time: start,
        end_time: end,
        keeper_fee: 0i128,
        is_revocable: true,
        is_transferable: false,
        step_duration: 0u64,
    };
    let action = AdminAction::AddBeneficiary(beneficiary.clone(), cfg);
    client.propose_admin_action(&admin, &action);
}

#[test]
#[should_panic(expected = "duration exceeds MAX_DURATION")]
fn create_vault_lazy_rejects_over_max_duration() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let beneficiary = Address::generate(&env);
    let start = env.ledger().timestamp();
    let end = start + MAX_DURATION + 1;

    let cfg = ScheduleConfig {
        owner: beneficiary.clone(),
        amount: 1_000i128,
        start_time: start,
        end_time: end,
        keeper_fee: 0i128,
        is_revocable: true,
        is_transferable: false,
        step_duration: 0u64,
    };
    let action = AdminAction::AddBeneficiary(beneficiary.clone(), cfg);
    client.propose_admin_action(&admin, &action);
}

#[test]
#[should_panic(expected = "duration exceeds MAX_DURATION")]
fn batch_create_vaults_rejects_over_max_duration() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let recipient = Address::generate(&env);
    let start = 100u64;
    let end = start + MAX_DURATION + 1;

    let batch = BatchCreateData {
        recipients: vec![&env, recipient],
        amounts: vec![&env, 1_000i128],
        start_times: vec![&env, start],
        end_times: vec![&env, end],
        keeper_fees: vec![&env, 0i128],
        step_durations: vec![&env, 0u64],
    };

    // convert batch into individual AddBeneficiary proposals (tests expect panic on invalid duration)
    for i in 0..batch.recipients.len() {
        let owner = batch.recipients.get(i).unwrap();
        let cfg = ScheduleConfig {
            owner: owner.clone(),
            amount: batch.amounts.get(i).unwrap(),
            start_time: batch.start_times.get(i).unwrap(),
            end_time: batch.end_times.get(i).unwrap(),
            keeper_fee: batch.keeper_fees.get(i).unwrap(),
            is_revocable: true,
            is_transferable: false,
            step_duration: batch.step_durations.get(i).unwrap_or(0),
        };
        let action = AdminAction::AddBeneficiary(owner.clone(), cfg);
        client.propose_admin_action(&admin, &action);
    }
}

#[test]
#[should_panic(expected = "duration exceeds MAX_DURATION")]
fn batch_add_schedules_rejects_over_max_duration() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let start = 100u64;
    let end = start + MAX_DURATION + 1;

    let schedules = vec![
        &env,
        ScheduleConfig {
            owner: Address::generate(&env),
            amount: 1_000i128,
            start_time: start,
            end_time: end,
            keeper_fee: 0i128,
            is_revocable: true,
            is_transferable: false,
            step_duration: 0u64,
        },
    ];

    for i in 0..schedules.len() {
        let s = schedules.get(i).unwrap();
        let action = AdminAction::AddBeneficiary(s.owner.clone(), s.clone());
        client.propose_admin_action(&admin, &action);
    }
}
