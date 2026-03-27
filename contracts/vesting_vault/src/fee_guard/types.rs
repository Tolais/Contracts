use soroban_sdk::contracttype;

#[contracttype]
pub struct FeeLimit {
    pub max_fee: i128,
}