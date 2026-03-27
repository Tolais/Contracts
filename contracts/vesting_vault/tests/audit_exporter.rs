use soroban_sdk::{Env, Address};

#[test]
fn test_export_claims() {
    let env = Env::default();

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    // simulate claims (pseudo depending on setup)

    // assert filtering works
}