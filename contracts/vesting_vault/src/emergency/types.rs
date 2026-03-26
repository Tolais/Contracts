use soroban_sdk::contracttype;
use soroban_sdk::Address;

#[contracttype]
pub struct EmergencyConfig {
    pub dao_members: Vec<Address>,
    pub cold_storage: Address,
}